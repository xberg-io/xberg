//! Structured text extraction with document semantics.
//!
//! This module provides structured extraction that preserves document structure
//! including paragraphs and formatting information (bold, italic, font size).
//!
//! # Note on PDF Spec Compliance
//!
//! This module has been refactored to focus on PDF-specification-compliant extraction.
//! The following non-spec-compliant heuristics have been removed:
//! - Header/heading detection (based on font size heuristics)
//! - List detection (based on marker patterns)
//! - Text alignment detection (based on position heuristics)
//!
//! These detections are not based on PDF structure but on assumptions about document
//! layout, which may not hold for all PDFs. For semantic extraction, applications
//! should use PDF structure tags (if available) or their own domain-specific logic.
//!
//! # Usage
//!
//! ```ignore
//! use xberg_native_pdf::PdfDocument;
//! use xberg_native_pdf::extractors::StructuredExtractor;
//!
//! let mut doc = PdfDocument::open("document.pdf")?;
//! let mut extractor = StructuredExtractor::new();
//! let structured = extractor.extract_page(&mut doc, 0)?;
//!
//! for element in structured.elements {
//!     match element {
//!         DocumentElement::Paragraph { text, .. } => {
//!             println!("P: {}", text);
//!         }
//!         _ => {} // Other element types for API compatibility only
//!     }
//! }
//! # Ok::<(), xberg_native_pdf::error::Error>(())
//! ```

use crate::document::PdfDocument;
use crate::error::{Error, Result};
use crate::geometry::Rect;
use crate::layout::TextBlock;
use serde::{Deserialize, Serialize};

/// A structured document with semantic elements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredDocument {
    /// Document elements in reading order
    pub elements: Vec<DocumentElement>,

    /// Page dimensions
    pub page_size: (f32, f32),

    /// Metadata
    pub metadata: DocumentMetadata,
}

/// Document element types with semantic meaning.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DocumentElement {
    /// Header/title element
    #[serde(rename = "header")]
    Header {
        /// Header level (1-6, where 1 is largest)
        level: u8,
        /// Text content
        text: String,
        /// Text styling
        style: TextStyle,
        /// Bounding box
        bbox: BoundingBox,
    },

    /// Paragraph element
    #[serde(rename = "paragraph")]
    Paragraph {
        /// Text content
        text: String,
        /// Text styling
        style: TextStyle,
        /// Bounding box
        bbox: BoundingBox,
        /// Text alignment (left, center, right, justified)
        alignment: TextAlignment,
    },

    /// List element (ordered or unordered)
    #[serde(rename = "list")]
    List {
        /// List items
        items: Vec<ListItem>,
        /// Whether list is ordered (numbered) or unordered (bullets)
        ordered: bool,
        /// Bounding box
        bbox: BoundingBox,
    },

    /// Table element (future enhancement)
    #[serde(rename = "table")]
    Table {
        /// Number of rows
        rows: usize,
        /// Number of columns
        cols: usize,
        /// Cell data (row-major order)
        cells: Vec<Vec<String>>,
        /// Bounding box
        bbox: BoundingBox,
    },
}

/// List item with optional nesting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListItem {
    /// Item text
    pub text: String,
    /// Item styling
    pub style: TextStyle,
    /// Nested list (if any)
    pub nested: Option<Box<DocumentElement>>,
    /// Bounding box
    pub bbox: BoundingBox,
}

/// Text styling information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextStyle {
    /// Font family name
    pub font_family: String,
    /// Font size in points
    pub font_size: f32,
    /// Bold text
    pub bold: bool,
    /// Italic text
    pub italic: bool,
    /// Text color (RGB 0.0-1.0)
    pub color: (f32, f32, f32),
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_family: "Unknown".to_string(),
            font_size: 12.0,
            bold: false,
            italic: false,
            color: (0.0, 0.0, 0.0),
        }
    }
}

/// Bounding box (x, y, width, height).
pub type BoundingBox = (f32, f32, f32, f32);

/// Text alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextAlignment {
    /// Left-aligned
    Left,
    /// Center-aligned
    Center,
    /// Right-aligned
    Right,
    /// Justified
    Justified,
}

/// Document metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    /// Total number of elements extracted
    pub element_count: usize,
    /// Number of headers
    pub header_count: usize,
    /// Number of paragraphs
    pub paragraph_count: usize,
    /// Number of lists
    pub list_count: usize,
    /// Number of tables
    pub table_count: usize,
}

/// Structured text extractor.
///
/// Converts positioned characters into semantic document elements.
#[allow(dead_code)]
pub struct StructuredExtractor {
    /// Configuration options
    config: ExtractorConfig,
}

/// Extraction configuration.
///
/// # Note
///
/// Header detection, list detection, and alignment detection have been removed
/// for PDF-specification compliance. These options are retained for API compatibility
/// but no longer have any effect.
#[derive(Debug, Clone)]
pub struct ExtractorConfig {
    /// (Deprecated) Minimum font size to consider as header
    /// This option no longer has any effect.
    #[allow(dead_code)]
    pub min_header_size: f32,
    /// (Deprecated) Maximum header levels to detect
    /// This option no longer has any effect.
    #[allow(dead_code)]
    pub max_header_levels: u8,
    /// (Deprecated) Vertical gap threshold for paragraph breaks
    /// This option no longer has any effect.
    #[allow(dead_code)]
    pub paragraph_gap_threshold: f32,
    /// (Deprecated) Enable list detection
    /// This option no longer has any effect.
    #[allow(dead_code)]
    pub detect_lists: bool,
    /// (Deprecated) Enable table detection
    /// This option no longer has any effect.
    #[allow(dead_code)]
    pub detect_tables: bool,
}

impl Default for ExtractorConfig {
    fn default() -> Self {
        Self {
            min_header_size: 14.0,
            max_header_levels: 6,
            paragraph_gap_threshold: 1.5,
            detect_lists: false,
            detect_tables: false,
        }
    }
}

impl StructuredExtractor {
    /// Create a new structured extractor with default configuration.
    pub fn new() -> Self {
        Self {
            config: ExtractorConfig::default(),
        }
    }

    /// Create a new structured extractor with custom configuration.
    pub fn with_config(config: ExtractorConfig) -> Self {
        Self { config }
    }

    /// Extract structured content from a page.
    ///
    /// # Arguments
    ///
    /// * `document` - The PDF document
    /// * `page_num` - Zero-based page number
    ///
    /// # Returns
    ///
    /// A structured document with paragraphs and formatting information.
    ///
    /// # Errors
    ///
    /// Returns an error if the page cannot be processed.
    ///
    /// # Note
    ///
    /// This method extracts text blocks from the PDF and converts them to paragraphs
    /// with formatting information (bold, italic, font size). Header detection, list
    /// detection, and alignment detection have been removed for PDF-spec compliance.
    pub fn extract_page(&mut self, document: &mut PdfDocument, page_num: u32) -> Result<StructuredDocument> {
        let spans = document.extract_spans(page_num as usize)?;

        if spans.is_empty() {
            return Ok(StructuredDocument {
                elements: Vec::new(),
                page_size: (0.0, 0.0),
                metadata: DocumentMetadata {
                    element_count: 0,
                    header_count: 0,
                    paragraph_count: 0,
                    list_count: 0,
                    table_count: 0,
                },
            });
        }

        let blocks = self.spans_to_blocks(&spans);

        let elements = self.blocks_to_simple_paragraphs(&blocks);

        let page_size = self.calculate_page_size_from_spans(&spans);
        let metadata = self.calculate_metadata(&elements);

        Ok(StructuredDocument {
            elements,
            page_size,
            metadata,
        })
    }

    /// Convert text spans to text blocks.
    ///
    /// Spans are already properly grouped by the PDF content stream operators,
    /// so we just convert the data structure.
    fn spans_to_blocks(&self, spans: &[crate::layout::TextSpan]) -> Vec<TextBlock> {
        spans
            .iter()
            .map(|span| TextBlock {
                chars: Vec::new(), // Not needed for structure detection ~keep
                bbox: span.bbox,
                text: span.text.clone(),
                avg_font_size: span.font_size,
                dominant_font: span.font_name.clone(),
                is_bold: span.font_weight == crate::layout::FontWeight::Bold,
                is_italic: span.is_italic,
                mcid: span.mcid,
                mcid_scope: span.mcid_scope.clone(),
                sequence: span.sequence,
                rotation_degrees: span.rotation_degrees,
            })
            .collect()
    }

    /// Calculate page size from text spans.
    fn calculate_page_size_from_spans(&self, spans: &[crate::layout::TextSpan]) -> (f32, f32) {
        if spans.is_empty() {
            return (0.0, 0.0);
        }

        let mut max_x = 0.0f32;
        let mut max_y = 0.0f32;

        for span in spans {
            max_x = max_x.max(span.bbox.x + span.bbox.width);
            max_y = max_y.max(span.bbox.y + span.bbox.height);
        }

        (max_x, max_y)
    }

    /// Convert all blocks to simple paragraphs without semantic classification.
    ///
    /// This method converts text blocks to paragraphs with formatting information only.
    /// Header detection, list detection, and alignment detection have been removed
    /// for PDF-specification compliance.
    fn blocks_to_simple_paragraphs(&self, blocks: &[TextBlock]) -> Vec<DocumentElement> {
        blocks
            .iter()
            .map(|block| DocumentElement::Paragraph {
                text: block.text.clone(),
                style: Self::block_to_text_style(block),
                bbox: Self::bbox_from_rect(block.bbox),
                alignment: TextAlignment::Left, // No alignment detection ~keep
            })
            .collect()
    }

    /// Convert TextBlock to TextStyle with bold/italic detection.
    fn block_to_text_style(block: &TextBlock) -> TextStyle {
        let bold = block.is_bold || block.dominant_font.contains("Bold");

        let italic = block.dominant_font.contains("Italic") || block.dominant_font.contains("Oblique");

        let color = block
            .chars
            .first()
            .map(|c| (c.color.r, c.color.g, c.color.b))
            .unwrap_or((0.0, 0.0, 0.0));

        TextStyle {
            font_family: block.dominant_font.to_string(),
            font_size: block.avg_font_size,
            bold,
            italic,
            color,
        }
    }

    /// Convert Rect to BoundingBox tuple.
    fn bbox_from_rect(rect: Rect) -> BoundingBox {
        (rect.x, rect.y, rect.width, rect.height)
    }

    /// Calculate page dimensions from characters.
    /// Calculate document metadata.
    fn calculate_metadata(&self, elements: &[DocumentElement]) -> DocumentMetadata {
        let mut header_count = 0;
        let mut paragraph_count = 0;
        let mut list_count = 0;
        let mut table_count = 0;

        for elem in elements {
            match elem {
                DocumentElement::Header { .. } => header_count += 1,
                DocumentElement::Paragraph { .. } => paragraph_count += 1,
                DocumentElement::List { .. } => list_count += 1,
                DocumentElement::Table { .. } => table_count += 1,
            }
        }

        DocumentMetadata {
            element_count: elements.len(),
            header_count,
            paragraph_count,
            list_count,
            table_count,
        }
    }
}

impl Default for StructuredExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl StructuredDocument {
    /// Convert to plain text (for backward compatibility).
    pub fn to_plain_text(&self) -> String {
        let mut text = String::new();

        for element in &self.elements {
            match element {
                DocumentElement::Header { text: t, .. } => {
                    if !text.is_empty() {
                        text.push('\n');
                    }
                    text.push_str(t);
                    text.push('\n');
                }
                DocumentElement::Paragraph { text: t, .. } => {
                    if !text.is_empty() {
                        text.push('\n');
                    }
                    text.push_str(t);
                }
                DocumentElement::List { items, .. } => {
                    for item in items {
                        text.push('\n');
                        text.push_str(&item.text);
                    }
                }
                DocumentElement::Table { cells, .. } => {
                    for row in cells {
                        text.push('\n');
                        text.push_str(&row.join("\t"));
                    }
                }
            }
        }

        text
    }

    /// Export to JSON.
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).map_err(|e| Error::ParseError {
            offset: 0,
            reason: format!("Failed to serialize to JSON: {}", e),
        })
    }
}
