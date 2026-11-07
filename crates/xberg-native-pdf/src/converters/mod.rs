//! Shared text-shaping helpers for the extraction pipeline.
//!
//! This module used to host the Markdown / HTML / Office output converters.
//! Those are gone; what remains is the post-processing and whitespace
//! normalisation the text extractors themselves depend on, plus
//! [`ConversionOptions`], which is still the options bag threaded through
//! extraction, XFA and the parallel page walker.

pub mod text_post_processor;
pub mod whitespace;

pub use text_post_processor::TextPostProcessor;
pub use whitespace::{normalize_whitespace, remove_page_artifacts};

// Re-export BoldMarkerBehavior from pipeline config (single source of truth) ~keep
pub use crate::pipeline::config::BoldMarkerBehavior;

/// Configuration for table formatting in markdown.
///
/// All formatting parameters are configurable with no magic numbers.
#[derive(Debug, Clone)]
pub struct TableFormatConfig {
    /// Include markdown table header separator (default: true)
    pub include_header_separator: bool,
    /// Spaces around cell content (default: 1)
    pub cell_padding: usize,
    /// Minimum column width in characters (default: 3)
    pub min_column_width: usize,
    /// Merge adjacent empty cells (default: true)
    pub merge_adjacent_empty_cells: bool,
    /// Preserve bold/italic formatting in cells (default: true)
    pub preserve_cell_formatting: bool,
    /// Text to use for empty cells (default: "-")
    pub empty_cell_text: String,
}

impl TableFormatConfig {
    /// Create a standard markdown table configuration.
    pub fn default() -> Self {
        Self {
            include_header_separator: true,
            cell_padding: 1,
            min_column_width: 3,
            merge_adjacent_empty_cells: true,
            preserve_cell_formatting: true,
            empty_cell_text: "-".to_string(),
        }
    }

    /// Create a compact markdown table configuration.
    pub fn compact() -> Self {
        Self {
            include_header_separator: true,
            cell_padding: 0,
            min_column_width: 1,
            merge_adjacent_empty_cells: true,
            preserve_cell_formatting: false,
            empty_cell_text: String::new(),
        }
    }

    /// Create a detailed markdown table configuration.
    pub fn detailed() -> Self {
        Self {
            include_header_separator: true,
            cell_padding: 2,
            min_column_width: 5,
            merge_adjacent_empty_cells: false,
            preserve_cell_formatting: true,
            empty_cell_text: "—".to_string(),
        }
    }

    /// Create a custom markdown table configuration.
    pub fn custom() -> Self {
        Self::default()
    }

    /// Set cell padding (builder pattern).
    pub fn with_cell_padding(mut self, padding: usize) -> Self {
        self.cell_padding = padding;
        self
    }

    /// Set minimum column width (builder pattern).
    pub fn with_min_column_width(mut self, width: usize) -> Self {
        self.min_column_width = width;
        self
    }

    /// Set empty cell text (builder pattern).
    pub fn with_empty_cell_text(mut self, text: &str) -> Self {
        self.empty_cell_text = text.to_string();
        self
    }
}

impl Default for TableFormatConfig {
    fn default() -> Self {
        TableFormatConfig::default()
    }
}

/// Options for converting PDF pages to different formats.
///
/// These options control how the conversion is performed, including
/// layout preservation, heading detection, image handling, etc.
///
/// # Examples
///
/// ```
/// use xberg_native_pdf::converters::{BoldMarkerBehavior, ConversionOptions, ReadingOrderMode};
///
/// // Default options
/// let opts = ConversionOptions::default();
///
/// // Custom options
/// let opts = ConversionOptions {
///     preserve_layout: true,
///     detect_headings: false,
///     extract_tables: false,
///     include_images: false,
///     image_output_dir: Some("images/".to_string()),
///     reading_order_mode: ReadingOrderMode::ColumnAware,
///     bold_marker_behavior: BoldMarkerBehavior::Conservative,
///     table_detection_config: None,
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ConversionOptions {
    /// Preserve exact layout with CSS positioning (HTML only).
    ///
    /// When true, generates HTML with absolute positioning to match the PDF layout.
    /// When false, generates semantic HTML with natural flow.
    pub preserve_layout: bool,

    /// Automatically detect headings based on font size and weight.
    ///
    /// When true, uses font clustering to identify heading levels (H1, H2, H3).
    /// When false, treats all text as paragraphs.
    pub detect_headings: bool,

    /// Extract tables from the document.
    ///
    /// Note: Table extraction is currently not fully implemented.
    pub extract_tables: bool,

    /// Include images in the output.
    ///
    /// When true, images are included as Markdown image syntax or HTML img tags.
    /// When false (default), images are omitted from the output.
    ///
    /// Defaults to `false` to keep output compact. Base64-embedded images can
    /// add hundreds of KB per page (e.g., 360 KB for a single-page invoice).
    /// Set to `true` when image content is needed.
    pub include_images: bool,

    /// Directory path for saving extracted images.
    ///
    /// If None, images are referenced but not saved.
    /// If Some(path), images are saved to the specified directory.
    pub image_output_dir: Option<String>,

    /// Embed images as base64 data URIs in output.
    ///
    /// When true (default), images are embedded directly as base64 data URIs.
    /// This creates self-contained files that don't require external image files.
    /// Works in HTML and Markdown (Obsidian, Typora, VS Code, Jupyter support base64).
    ///
    /// When false, images are saved to `image_output_dir` and referenced by path.
    /// Note: GitHub/GitLab Markdown renderers block base64 images for security.
    pub embed_images: bool,

    /// Strip repeated running headers/footers from untagged documents (WS2.6).
    ///
    /// When true, a cross-page pass finds top/bottom-band text lines that
    /// recur on a majority of pages (page numbers ignored) and drops them from
    /// the output — the geometric counterpart to the `/Artifact`-tag filtering
    /// that already handles tagged PDFs. Off by default (behaviour change).
    pub strip_running_headers_footers: bool,

    /// Reading order determination mode.
    ///
    /// Controls how text blocks are ordered in the output.
    pub reading_order_mode: ReadingOrderMode,

    /// Control how bold markers are applied in markdown conversion.
    ///
    /// Determines whether bold formatting markers are applied to whitespace-only
    /// content (Aggressive) or only to content-bearing text (Conservative).
    /// See BoldMarkerBehavior for details.
    pub bold_marker_behavior: BoldMarkerBehavior,

    /// Configuration for spatial table detection.
    ///
    /// If None, uses default configuration.
    /// Only applies when extract_tables = true.
    pub table_detection_config: Option<crate::structure::TableDetectionConfig>,

    /// Render formulas as embedded base64 images.
    ///
    /// When true and page_images are provided, formulas from the structure tree
    /// are cropped from rendered page images and embedded as base64 data URIs.
    /// Requires a Tagged PDF with Formula structure elements.
    pub render_formulas: bool,

    /// Paths to pre-rendered page images for formula extraction.
    ///
    /// Each path should point to a PNG image of the corresponding page.
    /// Index 0 = page 0, index 1 = page 1, etc.
    /// Required when render_formulas = true.
    pub page_images: Option<Vec<std::path::PathBuf>>,

    /// Page dimensions in PDF points (width, height).
    ///
    /// Required for coordinate conversion when render_formulas = true.
    /// Defaults to A4 (595.276 x 841.89) if not specified.
    pub page_dimensions: Option<(f32, f32)>,

    /// Include form field values inline in output.
    ///
    /// When true (default), form field values (text fields, checkboxes, choice fields)
    /// are converted to TextSpans at their spatial positions and merged with page content.
    /// This makes field values appear where they visually belong on the page.
    ///
    /// When false, form field values are omitted from output.
    pub include_form_fields: bool,

    /// Maximum image size (in pixels) to embed in output.
    ///
    /// Images exceeding this pixel count (width × height) are skipped when embedding
    /// as base64 data URIs or saving to files. This prevents oversized full-page scans
    /// from bloating output with hundreds of KB of encoded data per page.
    ///
    /// Common reference sizes:
    /// - A4 @ 150 DPI: ~1240 × 1754 = 2.2 MP
    /// - A4 @ 300 DPI: ~2480 × 3508 = 8.7 MP
    /// - A4 @ 600 DPI: ~4960 × 7016 = 34.8 MP
    ///
    /// Default: 16,000,000 (16 MP) — covers A4 at 300 DPI with margin.
    /// Set to `u64::MAX` to disable the limit entirely.
    /// Set to `0` to skip all images.
    pub max_image_pixels: Option<u64>,

    /// Rectangular regions to exclude from text extraction.
    ///
    /// Spans whose bounding boxes match any region under `exclude_regions_mode`
    /// are dropped before the text-assembly pipeline runs. Use this to strip
    /// figure captions, sidebars, or any other spatially-identified region from
    /// the extracted text stream.
    ///
    /// For Tagged PDFs the extractor already honours `/Artifact` marked-content
    /// sequences (PDF spec ISO 32000-1 §14.8.2.2). This field provides the same
    /// capability for untagged PDFs where spatial coordinates are the only way
    /// to identify non-body regions.
    ///
    /// Note: exclusion is unconditional — a span inside a region is dropped
    /// regardless of its structure-tree role. For `MinOverlap(t)`, `t` is the
    /// fraction of the *span's* area that must overlap the excluded region.
    ///
    /// Default: empty (no regions excluded).
    pub exclude_regions: Vec<crate::geometry::Rect>,

    /// Overlap rule used when matching spans against `exclude_regions`.
    ///
    /// Default: [`crate::layout::RectFilterMode::Intersects`] — drops any span with any overlap.
    pub exclude_regions_mode: crate::layout::RectFilterMode,

    /// Restrict text extraction to a single rectangular region.
    ///
    /// When `Some((rect, mode))`, only spans that match `rect` under `mode`
    /// are kept before the text-assembly pipeline runs. This powers
    /// [`crate::document::PdfDocument::extract_text_in_rect`] so it produces fully-assembled
    /// output (line breaks, tables, reading order) rather than a flat word
    /// stream.
    ///
    /// Applied after `exclude_regions` so exclusions take precedence.
    ///
    /// Default: `None` (all spans kept).
    pub include_region: Option<(crate::geometry::Rect, crate::layout::RectFilterMode)>,

    /// Emit a visible marker in the markdown/HTML for a page that produced no
    /// extractable text but is a scanned / image page (rasterised content with
    /// no usable text layer). Without it, ~half the pages of a scanned corpus
    /// render as silently-blank, with no indication that content was lost and
    /// OCR is required. The marker is a block-quote naming the page.
    ///
    /// Default: `true`. Set `false` to keep scanned pages blank.
    pub annotate_skipped_pages: bool,

    /// Include spans tagged `/Artifact` (running headers/footers, page
    /// numbers, watermarks; ISO 32000-1:2008 §14.8.2.2.1) in the output.
    ///
    /// Default **`true`** for backward compatibility with pre-0.3.42
    /// behavior — the same rationale already shipped on
    /// [`crate::document::PdfDocument::extract_words_with_thresholds`] /
    /// `extract_text_lines`: flipping the default would surface as a
    /// content regression on PDFs whose running-artifact heuristic
    /// over-triggers on real content (e.g. a repeated footer that carries
    /// a section identifier, not just decoration). Set `false` to get the
    /// spec-correct behavior (artifact-tagged spans excluded).
    pub include_artifacts: bool,
}

impl Default for ConversionOptions {
    /// Create default conversion options.
    ///
    /// Defaults:
    /// - preserve_layout: false (semantic mode)
    /// - detect_headings: true (enabled for proper markdown output)
    /// - extract_tables: true
    /// - include_images: false (opt-in to avoid output bloat from base64 images)
    /// - image_output_dir: None
    /// - embed_images: true (base64 for HTML, when include_images is enabled)
    /// - reading_order_mode: StructureTreeFirst (PDF-spec-compliant for Tagged PDFs, falls back to XY-Cut for untagged)
    /// - bold_marker_behavior: Conservative (no bold markers for whitespace-only content)
    /// - table_detection_config: None (uses defaults when table detection is enabled)
    /// - render_formulas: false
    /// - page_images: None
    /// - page_dimensions: None (defaults to A4 when needed)
    /// - include_form_fields: true
    /// - max_image_pixels: None (uses default 16 MP)
    fn default() -> Self {
        Self {
            preserve_layout: false,
            detect_headings: true,
            extract_tables: true,
            include_images: false,
            image_output_dir: None,
            embed_images: true,
            strip_running_headers_footers: false,
            reading_order_mode: ReadingOrderMode::StructureTreeFirst { mcid_order: vec![] },
            bold_marker_behavior: BoldMarkerBehavior::Conservative,
            table_detection_config: None,
            render_formulas: false,
            page_images: None,
            page_dimensions: None,
            include_form_fields: true,
            max_image_pixels: None,
            exclude_regions: Vec::new(),
            exclude_regions_mode: crate::layout::RectFilterMode::Intersects,
            include_region: None,
            annotate_skipped_pages: true,
            include_artifacts: true,
        }
    }
}

impl ConversionOptions {
    /// Enable table detection with custom configuration.
    ///
    /// Sets extract_tables = true and uses the provided configuration.
    ///
    /// # Examples
    ///
    /// ```
    /// use xberg_native_pdf::converters::ConversionOptions;
    /// use xberg_native_pdf::structure::TableDetectionConfig;
    ///
    /// let config = TableDetectionConfig::strict();
    /// let opts = ConversionOptions::default().with_table_detection(config);
    ///
    /// assert!(opts.extract_tables);
    /// assert!(opts.table_detection_config.is_some());
    /// ```
    pub fn with_table_detection(mut self, config: crate::structure::TableDetectionConfig) -> Self {
        self.extract_tables = true;
        self.table_detection_config = Some(config);
        self
    }

    /// Enable table detection with default configuration.
    ///
    /// Sets extract_tables = true and table_detection_config = None,
    /// which will use the default TableDetectionConfig when detection runs.
    ///
    /// # Examples
    ///
    /// ```
    /// use xberg_native_pdf::converters::ConversionOptions;
    ///
    /// let opts = ConversionOptions::default().with_default_table_detection();
    ///
    /// assert!(opts.extract_tables);
    /// assert!(opts.table_detection_config.is_none());
    /// ```
    pub fn with_default_table_detection(mut self) -> Self {
        self.extract_tables = true;
        self.table_detection_config = None;
        self
    }
}

/// Reading order determination mode for text blocks.
///
/// Determines how text blocks are ordered when converting to output formats.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadingOrderMode {
    /// Simple top-to-bottom, left-to-right ordering.
    ///
    /// Sorts all blocks by Y coordinate (top to bottom), then by X coordinate (left to right).
    /// This works well for single-column documents.
    TopToBottomLeftToRight,

    /// Column-aware reading order.
    ///
    /// Uses the XY-Cut algorithm to detect columns and determines proper reading order
    /// across multiple columns. This works better for multi-column documents.
    ColumnAware,

    /// Structure tree first, with fallback to column-aware.
    ///
    /// For Tagged PDFs: Uses the PDF logical structure tree (ISO 32000-1:2008 Section 14.7)
    /// to determine reading order via Marked Content IDs (MCIDs). This is the PDF-spec-compliant
    /// approach and provides perfect reading order for Tagged PDFs.
    ///
    /// For Untagged PDFs: Falls back to ColumnAware (XY-Cut algorithm).
    ///
    /// This mode requires passing MCID reading order through ConversionOptions.mcid_order.
    StructureTreeFirst {
        /// Reading order as a sequence of MCIDs from structure tree traversal.
        /// If empty, falls back to ColumnAware mode.
        mcid_order: Vec<u32>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversion_options_default() {
        let opts = ConversionOptions::default();
        assert!(!opts.preserve_layout);
        assert!(opts.detect_headings);
        assert!(opts.extract_tables);
        assert!(!opts.include_images);
        assert_eq!(opts.image_output_dir, None);
        assert!(opts.embed_images);
        assert_eq!(
            opts.reading_order_mode,
            ReadingOrderMode::StructureTreeFirst { mcid_order: vec![] }
        );
    }

    #[test]
    fn test_conversion_options_embed_images() {
        let opts = ConversionOptions::default();
        assert!(opts.embed_images);

        let opts = ConversionOptions {
            embed_images: false,
            image_output_dir: Some("images/".to_string()),
            ..Default::default()
        };
        assert!(!opts.embed_images);
        assert_eq!(opts.image_output_dir, Some("images/".to_string()));
    }

    #[test]
    fn test_conversion_options_custom() {
        let opts = ConversionOptions {
            preserve_layout: true,
            detect_headings: false,
            extract_tables: false,
            include_images: false,
            image_output_dir: Some("output/".to_string()),
            reading_order_mode: ReadingOrderMode::ColumnAware,
            bold_marker_behavior: BoldMarkerBehavior::Aggressive,
            table_detection_config: None,
            ..Default::default()
        };

        assert!(opts.preserve_layout);
        assert!(!opts.detect_headings);
        assert!(!opts.include_images);
        assert_eq!(opts.image_output_dir, Some("output/".to_string()));
        assert_eq!(opts.reading_order_mode, ReadingOrderMode::ColumnAware);
        assert_eq!(opts.bold_marker_behavior, BoldMarkerBehavior::Aggressive);
        assert!(opts.table_detection_config.is_none());
    }

    #[test]
    fn test_reading_order_mode_equality() {
        assert_eq!(
            ReadingOrderMode::TopToBottomLeftToRight,
            ReadingOrderMode::TopToBottomLeftToRight
        );
        assert_ne!(ReadingOrderMode::TopToBottomLeftToRight, ReadingOrderMode::ColumnAware);
    }

    #[test]
    fn test_conversion_options_clone() {
        let opts1 = ConversionOptions::default();
        let opts2 = opts1.clone();
        assert_eq!(opts1, opts2);
    }

    #[test]
    fn test_conversion_options_debug() {
        let opts = ConversionOptions::default();
        let debug_str = format!("{:?}", opts);
        assert!(debug_str.contains("ConversionOptions"));
    }

    #[test]
    fn test_bold_marker_behavior_default() {
        assert_eq!(BoldMarkerBehavior::default(), BoldMarkerBehavior::Conservative);
    }

    #[test]
    fn test_bold_marker_behavior_equality() {
        assert_eq!(BoldMarkerBehavior::Conservative, BoldMarkerBehavior::Conservative);
        assert_eq!(BoldMarkerBehavior::Aggressive, BoldMarkerBehavior::Aggressive);
        assert_ne!(BoldMarkerBehavior::Conservative, BoldMarkerBehavior::Aggressive);
    }

    #[test]
    fn test_bold_marker_behavior_copy_clone() {
        let behavior = BoldMarkerBehavior::Aggressive;
        let copied = behavior;
        assert_eq!(behavior, copied);
    }

    #[test]
    fn test_with_default_table_detection() {
        let opts = ConversionOptions::default().with_default_table_detection();
        assert!(opts.extract_tables);
        assert!(opts.table_detection_config.is_none());
    }

    #[test]
    fn test_with_table_detection() {
        let config = crate::structure::TableDetectionConfig::strict();
        let opts = ConversionOptions::default().with_table_detection(config);
        assert!(opts.extract_tables);
        assert!(opts.table_detection_config.is_some());
        let cfg = opts.table_detection_config.unwrap();
        assert_eq!(cfg.min_table_columns, 3);
        assert_eq!(cfg.column_tolerance, 2.0);
    }

    #[test]
    fn test_conversion_options_default_table_config() {
        let opts = ConversionOptions::default();
        // In v0.3.16, table extraction is enabled by default ~keep
        assert!(opts.extract_tables);
        // But detection config remains None (uses internal defaults) unless customized ~keep
        assert!(opts.table_detection_config.is_none());
    }

    #[test]
    fn test_include_images_default_false() {
        // Default should NOT include images to avoid output bloat from base64 embedding.
        // Users must explicitly opt in with include_images: true. ~keep
        let opts = ConversionOptions::default();
        assert!(
            !opts.include_images,
            "include_images should default to false to prevent bloated output"
        );
    }

    #[test]
    fn test_include_images_opt_in() {
        let opts = ConversionOptions {
            include_images: true,
            ..Default::default()
        };
        assert!(opts.include_images);
        // embed_images should still default to true (base64 when images are included) ~keep
        assert!(opts.embed_images);
    }
}
