//! PDF text hierarchy extraction and text block analysis.
//!
//! This module provides functions for extracting character information from PDFs,
//! merging characters into text blocks, and assigning hierarchy levels based on
//! font size analysis.

use serde::{Deserialize, Serialize};

use super::bounding_box::BoundingBox;
use crate::pdf::error::{PdfError, Result};
use pdfium_render::prelude::*;

// Magic number constants
const DEFAULT_FONT_SIZE: f32 = 12.0;
const MERGE_INTERSECTION_THRESHOLD: f32 = 0.05;
const MERGE_X_THRESHOLD_MULTIPLIER: f32 = 2.0;
const MERGE_Y_THRESHOLD_MULTIPLIER: f32 = 1.5;

/// Character information extracted from PDF with font metrics.
#[derive(Debug, Clone)]
pub struct CharData {
    /// The character text content
    pub text: String,
    /// X position in PDF units
    pub x: f32,
    /// Y position in PDF units
    pub y: f32,
    /// Font size in points
    pub font_size: f32,
    /// Character width in PDF units
    pub width: f32,
    /// Character height in PDF units
    pub height: f32,
}

/// A block of text with spatial and semantic information.
#[derive(Debug, Clone, PartialEq)]
pub struct TextBlock {
    /// The text content
    pub text: String,
    /// The bounding box of the block
    pub bbox: BoundingBox,
    /// The font size of the text in this block
    pub font_size: f32,
}

/// Result of KMeans clustering on font sizes.
///
/// Contains cluster labels for each block, where cluster index indicates
/// the hierarchy level: 0=H1, 1=H2, ..., 5=H6, 6+=Body.
#[derive(Debug, Clone)]
pub struct KMeansResult {
    /// Cluster label for each block (0-indexed)
    pub labels: Vec<u32>,
}

/// Hierarchy level assignment result.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HierarchyLevel {
    /// H1 - Top-level heading
    H1 = 1,
    /// H2 - Secondary heading
    H2 = 2,
    /// H3 - Tertiary heading
    H3 = 3,
    /// H4 - Quaternary heading
    H4 = 4,
    /// H5 - Quinary heading
    H5 = 5,
    /// H6 - Senary heading
    H6 = 6,
    /// Body text
    #[default]
    Body = 0,
}

/// A TextBlock with hierarchy level assignment.
#[derive(Debug, Clone)]
pub struct HierarchyBlock {
    /// The text content
    pub text: String,
    /// The bounding box of the block
    pub bbox: BoundingBox,
    /// The font size of the text in this block
    pub font_size: f32,
    /// The hierarchy level of this block (H1-H6 or Body)
    pub hierarchy_level: HierarchyLevel,
}

/// Assign hierarchy levels to text blocks based on KMeans clustering results.
///
/// Maps cluster indices to HTML heading levels (H1-H6) and body text:
/// - Cluster 0 → H1 (top-level heading)
/// - Cluster 1 → H2 (secondary heading)
/// - Cluster 2 → H3 (tertiary heading)
/// - Cluster 3 → H4 (quaternary heading)
/// - Cluster 4 → H5 (quinary heading)
/// - Cluster 5 → H6 (senary heading)
/// - Cluster 6+ → Body (body text)
///
/// # Arguments
///
/// * `blocks` - Slice of TextBlock objects to assign hierarchy levels to
/// * `kmeans_result` - KMeansResult containing cluster labels for each block
///
/// # Returns
///
/// Vector of tuples containing (original block info, hierarchy level)
///
/// # Example
///
/// ```rust,no_run
/// # #[cfg(feature = "pdf")]
/// # {
/// use kreuzberg::pdf::hierarchy::{TextBlock, BoundingBox, HierarchyLevel, assign_hierarchy_levels, KMeansResult};
///
/// let blocks = vec![
///     TextBlock {
///         text: "Title".to_string(),
///         bbox: BoundingBox { left: 0.0, top: 0.0, right: 100.0, bottom: 24.0 },
///         font_size: 24.0,
///     },
///     TextBlock {
///         text: "Body".to_string(),
///         bbox: BoundingBox { left: 0.0, top: 30.0, right: 100.0, bottom: 42.0 },
///         font_size: 12.0,
///     },
/// ];
///
/// let kmeans_result = KMeansResult {
///     labels: vec![0, 6],
/// };
///
/// let results = assign_hierarchy_levels(&blocks, &kmeans_result);
/// assert_eq!(results[0].hierarchy_level, HierarchyLevel::H1);
/// assert_eq!(results[1].hierarchy_level, HierarchyLevel::Body);
/// # }
/// ```
pub(crate) fn assign_hierarchy_levels(blocks: &[TextBlock], kmeans_result: &KMeansResult) -> Vec<HierarchyBlock> {
    if blocks.is_empty() || kmeans_result.labels.is_empty() {
        return Vec::new();
    }

    blocks
        .iter()
        .zip(kmeans_result.labels.iter())
        .map(|(block, &cluster_id)| {
            let hierarchy_level = match cluster_id {
                0 => HierarchyLevel::H1,
                1 => HierarchyLevel::H2,
                2 => HierarchyLevel::H3,
                3 => HierarchyLevel::H4,
                4 => HierarchyLevel::H5,
                5 => HierarchyLevel::H6,
                _ => HierarchyLevel::Body,
            };

            HierarchyBlock {
                text: block.text.clone(),
                bbox: block.bbox,
                font_size: block.font_size,
                hierarchy_level,
            }
        })
        .collect()
}

/// Extract characters with fonts from a PDF page.
///
/// Iterates through all characters on a page, extracting text, position,
/// and font size information. Characters are returned in page order.
///
/// # Arguments
///
/// * `page` - PDF page to extract characters from
///
/// # Returns
///
/// Vector of CharData objects containing text and positioning information.
///
/// # Example
///
/// ```rust,no_run
/// # #[cfg(feature = "pdf")]
/// # {
/// use kreuzberg::pdf::hierarchy::extract_chars_with_fonts;
/// use pdfium_render::prelude::*;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let pdfium = Pdfium::default();
/// let document = pdfium.load_pdf_from_file("example.pdf", None)?;
/// let page = document.pages().get(0)?;
/// let chars = extract_chars_with_fonts(&page)?;
/// # Ok(())
/// # }
/// # }
/// ```
pub(crate) fn extract_chars_with_fonts(page: &PdfPage) -> Result<Vec<CharData>> {
    let page_text = page
        .text()
        .map_err(|e| PdfError::TextExtractionFailed(format!("Failed to get page text: {}", e)))?;

    let chars = page_text.chars();
    let char_count = chars.len();
    let mut char_data_list = Vec::with_capacity(char_count);

    // Use indexed access instead of iterator to avoid potential PDFium issues
    for i in 0..char_count {
        let Ok(pdf_char) = chars.get(i) else {
            continue;
        };

        // Get character unicode - skip if not available
        let Some(ch) = pdf_char.unicode_char() else {
            continue;
        };

        // Get font size - use DEFAULT_FONT_SIZE if not available
        let font_size = pdf_char.unscaled_font_size().value;
        let font_size = if font_size > 0.0 { font_size } else { DEFAULT_FONT_SIZE };

        // Get character bounds - skip character if bounds not available
        let Ok(bounds) = pdf_char.loose_bounds() else {
            continue;
        };

        // Extract position and size information
        let char_data = CharData {
            text: ch.to_string(),
            x: bounds.left().value,
            y: bounds.bottom().value,
            width: bounds.width().value,
            height: bounds.height().value,
            font_size,
        };

        char_data_list.push(char_data);
    }

    Ok(char_data_list)
}

/// Text segment data extracted from PDF using pdfium's pre-merged segments.
///
/// Pdfium merges characters sharing the same baseline and font settings into segments,
/// providing correct word boundaries without gap-based heuristics. Each segment contains
/// the full text run, bounding box, and font metadata sampled from the first character.
#[derive(Debug, Clone)]
pub struct SegmentData {
    /// The segment text content (may contain spaces / multiple words)
    pub text: String,
    /// Left x position in PDF units
    pub x: f32,
    /// Bottom y position in PDF units (PDF coordinate system, y=0 at bottom)
    pub y: f32,
    /// Width of the segment bounding box
    pub width: f32,
    /// Height of the segment bounding box
    pub height: f32,
    /// Font size in points (from first character)
    pub font_size: f32,
    /// Whether the font is bold
    pub is_bold: bool,
    /// Whether the font is italic
    pub is_italic: bool,
    /// Whether the font is monospace (e.g. Courier, Consolas)
    pub is_monospace: bool,
    /// Baseline Y position (from first character origin, falls back to bounds bottom)
    pub baseline_y: f32,
    /// Pre-assigned heading level from the PDF structure tree (1-6), or `None`
    /// when the heading level is unknown and must be inferred via font-size clustering.
    pub assigned_role: Option<u8>,
}

/// Merge characters into text blocks using a greedy clustering algorithm.
///
/// Groups characters based on spatial proximity using weighted distance and
/// intersection ratio metrics. Characters are merged greedily based on their
/// proximity and overlap.
///
/// # Arguments
///
/// * `chars` - Vector of CharData to merge into blocks
///
/// # Returns
///
/// Vector of TextBlock objects containing merged characters
///
/// # Algorithm
///
/// The function uses a greedy approach:
/// 1. Create bounding boxes for each character
/// 2. Use per-axis distance thresholds based on font size
/// 3. Use intersection_ratio to detect overlapping or very close characters
/// 4. Merge characters into blocks based on proximity thresholds
/// 5. Return sorted blocks by position (top to bottom, left to right)
pub(crate) fn merge_chars_into_blocks(chars: Vec<CharData>) -> Vec<TextBlock> {
    if chars.is_empty() {
        return Vec::new();
    }

    // Create bounding boxes for each character
    let mut char_boxes: Vec<(CharData, BoundingBox)> = chars
        .into_iter()
        .map(|char_data| {
            let bbox = BoundingBox {
                left: char_data.x,
                top: char_data.y - char_data.height,
                right: char_data.x + char_data.width,
                bottom: char_data.y,
            };
            (char_data, bbox)
        })
        .collect();

    // Sort by position (top to bottom, then left to right)
    char_boxes.sort_by(|a, b| a.1.top.total_cmp(&b.1.top).then_with(|| a.1.left.total_cmp(&b.1.left)));

    // Greedy merging using union-find-like approach
    let mut blocks: Vec<Vec<CharData>> = Vec::new();
    let mut used = vec![false; char_boxes.len()];

    for i in 0..char_boxes.len() {
        if used[i] {
            continue;
        }

        let mut current_block = vec![char_boxes[i].0.clone()];
        let mut block_bbox = char_boxes[i].1;
        used[i] = true;

        // Try to merge with nearby characters
        let mut changed = true;
        while changed {
            changed = false;

            for j in (i + 1)..char_boxes.len() {
                if used[j] {
                    continue;
                }

                let next_char = &char_boxes[j];
                let next_bbox = char_boxes[j].1;

                // Calculate merge thresholds based on font size
                let avg_font_size = (block_bbox.bottom - block_bbox.top).max(next_bbox.bottom - next_bbox.top);

                let intersection_ratio = block_bbox.intersection_ratio(&next_bbox);

                // Check individual component distances
                let (self_center_x, self_center_y) = block_bbox.center();
                let (other_center_x, other_center_y) = next_bbox.center();
                let dx = (self_center_x - other_center_x).abs();
                let dy = (self_center_y - other_center_y).abs();

                // Separate thresholds for X and Y to handle different scenarios
                // Horizontal merging: allow up to 2-3 character widths apart (typical letter spacing)
                // Width per character ≈ 0.6 * font_size, spacing between chars ≈ 0.3 * font_size
                let x_threshold = avg_font_size * MERGE_X_THRESHOLD_MULTIPLIER;
                // Vertical merging: allow characters on same line (Y threshold is font height)
                let y_threshold = avg_font_size * MERGE_Y_THRESHOLD_MULTIPLIER;

                // Merge if close enough in both dimensions or overlapping
                let merge_by_distance = (dx < x_threshold) && (dy < y_threshold);
                if merge_by_distance || intersection_ratio > MERGE_INTERSECTION_THRESHOLD {
                    current_block.push(next_char.0.clone());
                    // Expand bounding box
                    block_bbox.left = block_bbox.left.min(next_bbox.left);
                    block_bbox.top = block_bbox.top.min(next_bbox.top);
                    block_bbox.right = block_bbox.right.max(next_bbox.right);
                    block_bbox.bottom = block_bbox.bottom.max(next_bbox.bottom);
                    used[j] = true;
                    changed = true;
                }
            }
        }

        blocks.push(current_block);
    }

    // Convert blocks to TextBlock objects
    blocks
        .into_iter()
        .map(|block| {
            let text = block.iter().map(|c| c.text.clone()).collect::<String>();

            // Calculate bounding box and average font size in a single fold operation
            let (min_x, min_y, max_x, max_y, total_font_size) = block.iter().fold(
                (f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY, 0.0),
                |(min_x, min_y, max_x, max_y, total_font_size), char_data| {
                    (
                        min_x.min(char_data.x),
                        min_y.min(char_data.y - char_data.height),
                        max_x.max(char_data.x + char_data.width),
                        max_y.max(char_data.y),
                        total_font_size + char_data.font_size,
                    )
                },
            );

            let avg_font_size = total_font_size / block.len() as f32;

            // Bounding box coordinates (allow negative values from PDFs)
            TextBlock {
                text,
                bbox: BoundingBox {
                    left: min_x,
                    top: min_y,
                    right: max_x,
                    bottom: max_y,
                },
                font_size: avg_font_size,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_char_data_creation() {
        let char_data = CharData {
            text: "A".to_string(),
            x: 100.0,
            y: 50.0,
            font_size: 12.0,
            width: 10.0,
            height: 12.0,
        };

        assert_eq!(char_data.text, "A");
        assert_eq!(char_data.x, 100.0);
        assert_eq!(char_data.y, 50.0);
        assert_eq!(char_data.font_size, 12.0);
        assert_eq!(char_data.width, 10.0);
        assert_eq!(char_data.height, 12.0);
    }

    #[test]
    fn test_char_data_clone() {
        let char_data = CharData {
            text: "B".to_string(),
            x: 200.0,
            y: 100.0,
            font_size: 14.0,
            width: 8.0,
            height: 14.0,
        };

        let cloned = char_data.clone();
        assert_eq!(cloned.text, char_data.text);
        assert_eq!(cloned.font_size, char_data.font_size);
    }
}
