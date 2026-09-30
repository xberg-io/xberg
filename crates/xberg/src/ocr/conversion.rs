//! Conversion utilities between OCR backend formats and unified OcrElement type.
//!
//! This module provides bidirectional conversion between:
//! - PaddleOCR `TextBlock` → `OcrElement`
//! - Tesseract TSV rows → `OcrElement`
//! - `OcrElement` → `HocrWord` (for table reconstruction)
//!
//! # Example
//!
//! ```rust,ignore
//! use xberg::ocr::conversion::{text_block_to_element, element_to_hocr_word};
//!
//! // Convert PaddleOCR result to unified element (with error handling)
//! let element = text_block_to_element(&text_block)?;
//!
//! // Convert back to HocrWord for table detection
//! let hocr_word = element_to_hocr_word(&element);
//! ```

#[cfg(paddle_ocr)]
use crate::types::OcrRotation;
use crate::types::{OcrBoundingGeometry, OcrConfidence, OcrElement, OcrElementLevel};

#[cfg(paddle_ocr)]
use crate::error::{Result, XbergError};

#[cfg(paddle_ocr)]
use xberg_paddle_ocr::{DetailedTextBlock, TextBlock, WordBlock};

#[cfg(paddle_ocr)]
use super::table::HocrWord;

/// Convert a PaddleOCR TextBlock to a unified OcrElement.
///
/// Preserves all spatial information including:
/// - 4-point quadrilateral bounding box
/// - Detection and recognition confidence scores
/// - Rotation angle and confidence
///
/// # Arguments
///
/// * `block` - PaddleOCR TextBlock containing OCR results
/// * `page_number` - 1-indexed page number
///
/// # Returns
///
/// A fully populated `OcrElement` with all available metadata.
///
/// # Errors
///
/// Returns an error if:
/// - `box_points` has fewer than 4 points (malformed detection)
/// - `angle_index` is outside the valid range (0-3)
///
/// Returns `Ok(None)` if the detection is filtered out due to low `box_score`.
#[cfg(paddle_ocr)]
pub(crate) fn text_block_to_element(block: &TextBlock, page_number: u32) -> Result<Option<OcrElement>> {
    if block.box_score < 0.3 {
        return Ok(None);
    }

    if block.box_points.len() < 4 {
        return Err(XbergError::ocr(format!(
            "PaddleOCR TextBlock has {} box_points, expected at least 4. This indicates malformed OCR output.",
            block.box_points.len()
        )));
    }

    let points: [(u32, u32); 4] = [
        (block.box_points[0].x, block.box_points[0].y),
        (block.box_points[1].x, block.box_points[1].y),
        (block.box_points[2].x, block.box_points[2].y),
        (block.box_points[3].x, block.box_points[3].y),
    ];

    let geometry = OcrBoundingGeometry::Quadrilateral {
        points: points.into_iter().map(Into::into).collect(),
    };

    let confidence = OcrConfidence::from_paddle(block.box_score, block.text_score);

    let rotation = if block.angle_index >= 0 {
        match OcrRotation::from_paddle(block.angle_index, block.angle_score) {
            Ok(rot) => Some(rot),
            Err(msg) => return Err(XbergError::ocr(msg)),
        }
    } else {
        None
    };

    Ok(Some(
        OcrElement::new(block.text.clone(), geometry, confidence)
            .with_level(OcrElementLevel::Line)
            .with_page_number(page_number)
            .with_rotation_opt(rotation)
            .with_metadata("backend", serde_json::json!("paddle-ocr")),
    ))
}

/// Line and word elements derived from one PaddleOCR recognition block.
#[cfg(paddle_ocr)]
pub(crate) struct PaddleElementGroup {
    pub(crate) line: OcrElement,
    pub(crate) words: Vec<OcrElement>,
}

/// Convert detailed PaddleOCR output without mixing semantic line text and word geometry.
#[cfg(paddle_ocr)]
pub(crate) fn detailed_text_block_to_elements(
    block: &DetailedTextBlock,
    page_number: u32,
) -> Result<Option<PaddleElementGroup>> {
    let Some(line) = text_block_to_element(&block.block, page_number)? else {
        return Ok(None);
    };
    let words = block
        .words
        .iter()
        .filter_map(|word| word_block_to_element(word, &line, block.block.box_score, page_number))
        .collect();
    Ok(Some(PaddleElementGroup { line, words }))
}

#[cfg(paddle_ocr)]
fn word_block_to_element(
    word: &WordBlock,
    line: &OcrElement,
    detection_confidence: f32,
    page_number: u32,
) -> Option<OcrElement> {
    let points = word.box_points.get(..4)?;
    if word.word.text.trim().is_empty() {
        return None;
    }
    let geometry = OcrBoundingGeometry::Quadrilateral {
        points: vec![
            (points[0].x, points[0].y).into(),
            (points[1].x, points[1].y).into(),
            (points[2].x, points[2].y).into(),
            (points[3].x, points[3].y).into(),
        ],
    };
    Some(
        OcrElement::new(
            word.word.text.clone(),
            geometry,
            OcrConfidence::from_paddle(detection_confidence, word.word.confidence),
        )
        .with_level(OcrElementLevel::Word)
        .with_page_number(page_number)
        .with_rotation_opt(line.rotation.clone())
        .with_metadata("backend", serde_json::json!("paddle-ocr")),
    )
}

/// Maximum vertical gap between two consecutive PaddleOCR lines, expressed as a
/// multiple of the running median line height seen so far on the page, for the
/// lines to be grouped into the same paragraph block. Relative to line height
/// (not an absolute pixel count) so the grouping is stable whether the page was
/// rasterized at 150 DPI or 450 DPI — an absolute-pixel version of this same
/// mistake is a known live defect elsewhere in this codebase
/// (`crates/xberg-paddle-ocr/src/ocr_lite.rs:528-542`, `VISUAL_LINE_Y_TOLERANCE_PX`).
#[cfg(paddle_ocr)]
const MAX_BLOCK_VERTICAL_GAP_IN_LINE_HEIGHTS: f64 = 0.6;

/// Minimum fraction of the narrower of two consecutive lines' horizontal extent
/// that must overlap for them to be considered part of the same paragraph
/// column.
#[cfg(paddle_ocr)]
const MIN_BLOCK_HORIZONTAL_OVERLAP_RATIO: f64 = 0.3;

/// Maximum left-edge offset between two consecutive lines, expressed as a
/// multiple of the running median line height, allowed as a fallback when the
/// lines don't x-overlap enough on their own (covers short lines that still
/// share a left margin with the paragraph they continue).
#[cfg(paddle_ocr)]
const MAX_BLOCK_LEFT_EDGE_OFFSET_IN_LINE_HEIGHTS: f64 = 0.5;

/// Minimum left-edge offset of a line beyond the running left margin of its
/// candidate block, expressed as a multiple of the running median line
/// height, for that line to be treated as an indented first line of a new
/// paragraph — overriding an otherwise-passing vertical-gap/overlap check.
///
/// Real single-column scanned prose usually signals a new paragraph purely
/// through first-line indentation, not through extra vertical whitespace:
/// the leading between the last line of one paragraph and the first line of
/// the next is identical to the leading between two lines of the same
/// paragraph. A purely vertical-gap-based check can therefore never split
/// such a page into more than one block, which is exactly the observed
/// defect (a 16-page scanned document producing exactly 16 paddle-ocr
/// paragraphs, one full-page block per page).
#[cfg(paddle_ocr)]
const MIN_PARAGRAPH_INDENT_IN_LINE_HEIGHTS: f64 = 0.75;

/// Axis-aligned bounds of one OCR line, derived from its (possibly
/// quadrilateral) geometry, used only for block-grouping decisions.
#[cfg(paddle_ocr)]
#[derive(Clone, Copy)]
struct LineBounds {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
}

#[cfg(paddle_ocr)]
impl LineBounds {
    fn from_geometry(geometry: &OcrBoundingGeometry) -> Self {
        let (left, top, width, height) = geometry.to_aabb();
        Self {
            left: f64::from(left),
            top: f64::from(top),
            right: f64::from(left + width),
            bottom: f64::from(top + height),
        }
    }

    fn height(&self) -> f64 {
        self.bottom - self.top
    }
}

/// Group PaddleOCR's per-line elements into paragraph blocks using their own
/// quadrilateral geometry, and return a stable block id per input element in
/// the same order.
///
/// PaddleOCR detects text line-by-line with no paragraph concept, unlike
/// Tesseract's hOCR which nests lines inside an `ocr_par`. Without this, every
/// PaddleOCR line becomes its own `PdfParagraph` in
/// `pdf::structure::adapters::ocr_doc_to_paragraphs`, which merges consecutive
/// elements only when they share an equal, non-empty block id (#631).
///
/// Ids are assigned in encounter order (`paddle-block-1`, `paddle-block-2`,
/// ...); consecutive lines join the running block when both their vertical gap
/// and horizontal alignment, each measured relative to the running median line
/// height, stay within tolerance.
#[cfg(paddle_ocr)]
pub(crate) fn assign_line_block_ids(elements: &[OcrElement]) -> Vec<String> {
    let mut block_ids = Vec::with_capacity(elements.len());
    let mut heights_seen: Vec<f64> = Vec::with_capacity(elements.len());
    let mut block_index: u32 = 0;
    let mut previous: Option<LineBounds> = None;
    // Left margin established so far by the current block's lines, used to
    // detect an indented paragraph start (see `starts_indented_paragraph`).
    let mut block_left_margin: f64 = 0.0;

    for element in elements {
        let bounds = LineBounds::from_geometry(&element.geometry);
        insert_sorted(&mut heights_seen, bounds.height());
        let median_line_height = running_median(&heights_seen);

        let continues_block = previous.as_ref().is_some_and(|previous| {
            lines_share_block(previous, &bounds, median_line_height)
                && !starts_indented_paragraph(bounds.left, block_left_margin, median_line_height)
        });

        if continues_block {
            block_left_margin = block_left_margin.min(bounds.left);
        } else {
            block_index += 1;
            block_left_margin = bounds.left;
        }
        block_ids.push(format!("paddle-block-{block_index}"));
        previous = Some(bounds);
    }

    block_ids
}

/// Whether `left` is indented far enough past `block_left_margin` — the
/// flush-left margin established by the current block's lines so far — to be
/// treated as a new paragraph's first line, in units of `median_line_height`.
///
/// This is independent of (and can override) the vertical-gap/overlap check
/// in [`lines_share_block`], because real body text often marks a paragraph
/// break with indentation alone and no extra vertical space (see
/// [`MIN_PARAGRAPH_INDENT_IN_LINE_HEIGHTS`]).
#[cfg(paddle_ocr)]
fn starts_indented_paragraph(left: f64, block_left_margin: f64, median_line_height: f64) -> bool {
    if median_line_height <= 0.0 {
        return false;
    }
    left - block_left_margin > median_line_height * MIN_PARAGRAPH_INDENT_IN_LINE_HEIGHTS
}

#[cfg(paddle_ocr)]
fn lines_share_block(previous: &LineBounds, current: &LineBounds, median_line_height: f64) -> bool {
    if median_line_height <= 0.0 {
        return false;
    }

    let vertical_gap = if current.top >= previous.bottom {
        current.top - previous.bottom
    } else if previous.top >= current.bottom {
        previous.top - current.bottom
    } else {
        0.0
    };
    if vertical_gap > median_line_height * MAX_BLOCK_VERTICAL_GAP_IN_LINE_HEIGHTS {
        return false;
    }

    let overlap = (previous.right.min(current.right) - previous.left.max(current.left)).max(0.0);
    let narrower_width = (previous.right - previous.left).min(current.right - current.left);
    let overlap_ratio = if narrower_width > 0.0 {
        overlap / narrower_width
    } else {
        0.0
    };
    let left_edge_offset = (previous.left - current.left).abs();

    overlap_ratio >= MIN_BLOCK_HORIZONTAL_OVERLAP_RATIO
        || left_edge_offset <= median_line_height * MAX_BLOCK_LEFT_EDGE_OFFSET_IN_LINE_HEIGHTS
}

/// Insert `value` into `sorted` (ascending), keeping it sorted, so a running
/// median can be read back after each insertion in O(log n) + O(n).
#[cfg(paddle_ocr)]
fn insert_sorted(sorted: &mut Vec<f64>, value: f64) {
    let index = sorted.partition_point(|existing| *existing < value);
    sorted.insert(index, value);
}

#[cfg(paddle_ocr)]
fn running_median(sorted: &[f64]) -> f64 {
    let len = sorted.len();
    if len == 0 {
        return 0.0;
    }
    if len % 2 == 1 {
        sorted[len / 2]
    } else {
        (sorted[len / 2 - 1] + sorted[len / 2]) / 2.0
    }
}

/// Tesseract TSV row data for conversion.
///
/// This struct represents a single row from Tesseract's TSV output format.
/// TSV format includes hierarchical information (block, paragraph, line, word)
/// along with bounding boxes and confidence scores.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone)]
pub struct TsvRow {
    /// Hierarchical level (1=block, 2=para, 3=line, 4=word, 5=symbol)
    pub level: i32,
    /// Page number (1-indexed)
    pub page_num: i32,
    /// Block number within page
    pub block_num: i32,
    /// Paragraph number within block
    pub par_num: i32,
    /// Line number within paragraph
    pub line_num: i32,
    /// Word number within line
    pub word_num: i32,
    /// Left x-coordinate in pixels
    pub left: u32,
    /// Top y-coordinate in pixels
    pub top: u32,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Confidence score (0-100)
    pub conf: f64,
    /// Recognized text
    pub text: String,
}

/// Convert a Tesseract TSV row to a unified OcrElement.
///
/// Preserves:
/// - Axis-aligned bounding box
/// - Recognition confidence (Tesseract doesn't have separate detection confidence)
/// - Hierarchical level information
///
/// # Arguments
///
/// * `row` - Parsed TSV row from Tesseract output
///
/// # Returns
///
/// An `OcrElement` with rectangle geometry and Tesseract metadata.
#[cfg(feature = "ocr")]
pub(crate) fn tsv_row_to_element(row: &TsvRow) -> OcrElement {
    let geometry = OcrBoundingGeometry::Rectangle {
        left: row.left,
        top: row.top,
        width: row.width,
        height: row.height,
    };

    let confidence = OcrConfidence::from_tesseract(row.conf);
    let level = OcrElementLevel::from_tesseract_level(row.level);

    OcrElement::new(row.text.clone(), geometry, confidence)
        .with_level(level)
        .with_page_number(row.page_num as u32)
        .with_metadata("backend", serde_json::json!("tesseract"))
        .with_metadata("block_num", serde_json::json!(row.block_num))
        .with_metadata("par_num", serde_json::json!(row.par_num))
        .with_metadata("line_num", serde_json::json!(row.line_num))
        .with_metadata("word_num", serde_json::json!(row.word_num))
}

/// Convert a Tesseract iterator WordData to a unified OcrElement with rich metadata.
///
/// Unlike `tsv_row_to_element` which only has text, bbox, and confidence,
/// this populates font attributes (bold, italic, monospace, pointsize) and
/// block/paragraph context from the Tesseract layout analysis.
///
/// # Arguments
///
/// * `word` - WordData from the Tesseract result iterator
/// * `block_type` - Optional block type from Tesseract layout analysis
/// * `para_info` - Optional paragraph metadata (justification, list item flag)
/// * `page_number` - 1-indexed page number
///
/// # Returns
///
/// An `OcrElement` at `Word` level with all available font and layout metadata.
#[cfg(feature = "ocr")]
pub(crate) fn iterator_word_to_element(
    word: &xberg_tesseract::WordData,
    block_type: Option<xberg_tesseract::TessPolyBlockType>,
    para_info: Option<&xberg_tesseract::ParaInfo>,
    page_number: u32,
) -> OcrElement {
    let left = word.left.max(0) as u32;
    let top = word.top.max(0) as u32;
    let width = (word.right - word.left).max(0) as u32;
    let height = (word.bottom - word.top).max(0) as u32;

    let geometry = OcrBoundingGeometry::Rectangle {
        left,
        top,
        width,
        height,
    };
    let confidence = OcrConfidence::from_tesseract(word.confidence as f64);

    let mut element = OcrElement::new(word.text.clone(), geometry, confidence)
        .with_level(OcrElementLevel::Word)
        .with_page_number(page_number)
        .with_metadata("backend", serde_json::json!("tesseract-iterator"));

    if let Some(ref attrs) = word.font_attrs {
        element = element
            .with_metadata("is_bold", serde_json::json!(attrs.is_bold))
            .with_metadata("is_italic", serde_json::json!(attrs.is_italic))
            .with_metadata("is_underlined", serde_json::json!(attrs.is_underlined))
            .with_metadata("is_monospace", serde_json::json!(attrs.is_monospace))
            .with_metadata("is_serif", serde_json::json!(attrs.is_serif))
            .with_metadata("is_smallcaps", serde_json::json!(attrs.is_smallcaps));
        if attrs.pointsize > 0 {
            element = element.with_metadata("pointsize", serde_json::json!(attrs.pointsize));
        }
        // font_id is an opaque per-document index into Tesseract's font table (not
        // a stable identifier across documents), but it lets callers group words
        // rendered with the same font without re-deriving that from pointsize/bold/italic.
        if attrs.font_id >= 0 {
            element = element.with_metadata("font_id", serde_json::json!(attrs.font_id));
        }
    }

    if let Some(ref lang) = word.language {
        element = element.with_metadata("word_language", serde_json::json!(lang));
    }

    if let Some(bt) = block_type {
        let block_type_str = match bt {
            xberg_tesseract::TessPolyBlockType::PT_UNKNOWN => "PT_UNKNOWN",
            xberg_tesseract::TessPolyBlockType::PT_FLOWING_TEXT => "PT_FLOWING_TEXT",
            xberg_tesseract::TessPolyBlockType::PT_HEADING_TEXT => "PT_HEADING_TEXT",
            xberg_tesseract::TessPolyBlockType::PT_PULLOUT_TEXT => "PT_PULLOUT_TEXT",
            xberg_tesseract::TessPolyBlockType::PT_EQUATION => "PT_EQUATION",
            xberg_tesseract::TessPolyBlockType::PT_INLINE_EQUATION => "PT_INLINE_EQUATION",
            xberg_tesseract::TessPolyBlockType::PT_TABLE => "PT_TABLE",
            xberg_tesseract::TessPolyBlockType::PT_VERTICAL_TEXT => "PT_VERTICAL_TEXT",
            xberg_tesseract::TessPolyBlockType::PT_CAPTION_TEXT => "PT_CAPTION_TEXT",
            xberg_tesseract::TessPolyBlockType::PT_FLOWING_IMAGE => "PT_FLOWING_IMAGE",
            xberg_tesseract::TessPolyBlockType::PT_HEADING_IMAGE => "PT_HEADING_IMAGE",
            xberg_tesseract::TessPolyBlockType::PT_PULLOUT_IMAGE => "PT_PULLOUT_IMAGE",
            xberg_tesseract::TessPolyBlockType::PT_HORZ_LINE => "PT_HORZ_LINE",
            xberg_tesseract::TessPolyBlockType::PT_VERT_LINE => "PT_VERT_LINE",
            xberg_tesseract::TessPolyBlockType::PT_NOISE => "PT_NOISE",
            xberg_tesseract::TessPolyBlockType::PT_COUNT => "PT_COUNT",
        };
        element = element.with_metadata("block_type", serde_json::json!(block_type_str));
    }

    if let Some(para) = para_info {
        let justification_str = match para.justification {
            xberg_tesseract::TessParagraphJustification::JUSTIFICATION_UNKNOWN => "unknown",
            xberg_tesseract::TessParagraphJustification::JUSTIFICATION_LEFT => "left",
            xberg_tesseract::TessParagraphJustification::JUSTIFICATION_CENTER => "center",
            xberg_tesseract::TessParagraphJustification::JUSTIFICATION_RIGHT => "right",
        };
        element = element
            .with_metadata("is_list_item", serde_json::json!(para.is_list_item))
            .with_metadata("is_crown", serde_json::json!(para.is_crown))
            .with_metadata("justification", serde_json::json!(justification_str));
        if para.first_line_indent != 0 {
            element = element.with_metadata("first_line_indent", serde_json::json!(para.first_line_indent));
        }
    }

    element
}

/// One line element per Tesseract text line, followed by the words themselves.
///
/// `words` is in reading order and pairs each word with the key of its text line, so a line is a
/// run of equal keys. Tesseract reports no text or confidence for a line, so the line joins its
/// words' text, spans the union of their boxes and takes their word-count-weighted mean
/// confidence. ~keep
#[cfg(feature = "ocr")]
pub(crate) fn tesseract_elements_with_lines<K: PartialEq>(words: Vec<(K, OcrElement)>) -> Vec<OcrElement> {
    let mut elements = words
        .chunk_by(|(left, _), (right, _)| left == right)
        .map(tesseract_line_element)
        .collect::<Vec<_>>();
    elements.extend(words.into_iter().map(|(_, word)| word));
    elements
}

#[cfg(feature = "ocr")]
fn tesseract_line_element<K>(words: &[(K, OcrElement)]) -> OcrElement {
    let (mut left, mut top, mut right, mut bottom) = (u32::MAX, u32::MAX, 0u32, 0u32);
    let (mut weighted_recognition, mut word_count) = (0.0, 0usize);
    for (_, word) in words {
        let (word_left, word_top, width, height) = word.geometry.to_aabb();
        left = left.min(word_left);
        top = top.min(word_top);
        right = right.max(word_left.saturating_add(width));
        bottom = bottom.max(word_top.saturating_add(height));
        let count = word.text.split_whitespace().count();
        weighted_recognition += word.confidence.recognition * count as f64;
        word_count += count;
    }
    let text = words
        .iter()
        .map(|(_, word)| word.text.trim())
        .collect::<Vec<_>>()
        .join(" ");
    let geometry = OcrBoundingGeometry::Rectangle {
        left,
        top,
        width: right.saturating_sub(left),
        height: bottom.saturating_sub(top),
    };
    let confidence = OcrConfidence {
        detection: None,
        recognition: if word_count == 0 {
            0.0
        } else {
            weighted_recognition / word_count as f64
        },
    };
    let first = &words[0].1;
    let mut line = OcrElement::new(text, geometry, confidence)
        .with_level(OcrElementLevel::Line)
        .with_page_number(first.page_number);
    if let Some(backend) = first.backend_metadata.get("backend") {
        line = line.with_metadata("backend", backend.clone());
    }
    line
}

/// Convert an OcrElement to an HocrWord for table reconstruction.
///
/// This enables reuse of the existing table detection algorithms from
/// html-to-markdown-rs with PaddleOCR results.
///
/// # Arguments
///
/// * `element` - Unified OCR element with geometry and text
///
/// # Returns
///
/// An `HocrWord` suitable for table reconstruction algorithms.
#[cfg(paddle_ocr)]
pub(crate) fn element_to_hocr_word(element: &OcrElement) -> HocrWord {
    let (left, top, width, height) = element.geometry.to_aabb();

    HocrWord {
        text: element.text.clone(),
        left,
        top,
        width,
        height,
        confidence: element.confidence.recognition * 100.0,
    }
}

/// Convert a vector of OcrElements to HocrWords for batch table processing.
///
/// Prefers word-level elements globally, as table reconstruction works best
/// with word-level granularity, and falls back to line-level elements when no
/// words satisfy the confidence threshold.
///
/// # Arguments
///
/// * `elements` - Slice of OCR elements to convert
/// * `min_confidence` - Minimum recognition confidence threshold (0.0-1.0)
///
/// # Returns
///
/// A vector of HocrWords filtered by confidence and element level.
#[cfg(paddle_ocr)]
pub(crate) fn elements_to_hocr_words(elements: &[OcrElement], min_confidence: f64) -> Vec<HocrWord> {
    let has_valid_words = elements
        .iter()
        .any(|element| element.confidence.recognition >= min_confidence && element.level == OcrElementLevel::Word);

    elements
        .iter()
        .filter(|element| element.confidence.recognition >= min_confidence)
        // Mixing line and word boxes duplicates text and corrupts table geometry. ~keep
        .filter(|element| {
            if has_valid_words {
                element.level == OcrElementLevel::Word
            } else {
                element.level == OcrElementLevel::Line
            }
        })
        .map(element_to_hocr_word)
        .collect()
}

/// Extension trait to add optional rotation to OcrElement builder.
#[cfg(paddle_ocr)]
trait OcrElementExt {
    fn with_rotation_opt(self, rotation: Option<OcrRotation>) -> Self;
}

#[cfg(paddle_ocr)]
impl OcrElementExt for OcrElement {
    #[cfg_attr(alef, alef(skip))]
    fn with_rotation_opt(mut self, rotation: Option<OcrRotation>) -> Self {
        self.rotation = rotation;
        self
    }
}

#[cfg(all(test, feature = "ocr"))]
mod tests {
    use super::*;

    #[test]
    fn test_tsv_row_to_element() {
        let row = TsvRow {
            level: 5,
            page_num: 1,
            block_num: 1,
            par_num: 1,
            line_num: 2,
            word_num: 3,
            left: 100,
            top: 50,
            width: 80,
            height: 20,
            conf: 95.0,
            text: "Hello".to_string(),
        };

        let element = tsv_row_to_element(&row);

        assert_eq!(element.text, "Hello");
        assert_eq!(element.level, OcrElementLevel::Word);
        assert_eq!(element.page_number, 1);
        assert_eq!(element.parent_id, None);

        #[cfg(any(paddle_ocr, feature = "layout-detection"))]
        {
            let (left, top, width, height) = element.geometry.to_aabb();
            assert_eq!((left, top, width, height), (100, 50, 80, 20));
        }

        assert!((element.confidence.recognition - 0.95).abs() < 0.001);
        assert!(element.confidence.detection.is_none());
    }

    #[cfg(paddle_ocr)]
    #[test]
    fn test_element_to_hocr_word() {
        let geometry = OcrBoundingGeometry::Rectangle {
            left: 100,
            top: 50,
            width: 80,
            height: 20,
        };
        let confidence = OcrConfidence::from_tesseract(92.5);
        let element = OcrElement::new("World", geometry, confidence);

        let word = element_to_hocr_word(&element);

        assert_eq!(word.text, "World");
        assert_eq!(word.left, 100);
        assert_eq!(word.top, 50);
        assert_eq!(word.width, 80);
        assert_eq!(word.height, 20);
        assert!((word.confidence - 92.5).abs() < 0.001);
    }

    #[cfg(paddle_ocr)]
    #[test]
    fn test_quadrilateral_to_hocr_word() {
        let geometry = OcrBoundingGeometry::Quadrilateral {
            points: [(10, 22), (108, 20), (110, 72), (12, 74)]
                .into_iter()
                .map(Into::into)
                .collect(),
        };
        let confidence = OcrConfidence::from_paddle(0.95, 0.88);
        let element = OcrElement::new("Rotated", geometry, confidence);

        let word = element_to_hocr_word(&element);

        assert_eq!(word.left, 10);
        assert_eq!(word.top, 20);
        assert_eq!(word.width, 100);
        assert_eq!(word.height, 54);

        assert!((word.confidence - 88.0).abs() < 0.1);
    }

    #[cfg(paddle_ocr)]
    #[test]
    fn test_elements_to_hocr_words_filtering() {
        let elements = vec![
            OcrElement::new(
                "word1",
                OcrBoundingGeometry::Rectangle {
                    left: 0,
                    top: 0,
                    width: 50,
                    height: 20,
                },
                OcrConfidence::from_tesseract(90.0),
            )
            .with_level(OcrElementLevel::Word),
            OcrElement::new(
                "word2",
                OcrBoundingGeometry::Rectangle {
                    left: 60,
                    top: 0,
                    width: 50,
                    height: 20,
                },
                OcrConfidence::from_tesseract(50.0),
            )
            .with_level(OcrElementLevel::Word),
            OcrElement::new(
                "block",
                OcrBoundingGeometry::Rectangle {
                    left: 0,
                    top: 0,
                    width: 200,
                    height: 100,
                },
                OcrConfidence::from_tesseract(95.0),
            )
            .with_level(OcrElementLevel::Block),
        ];

        let words = elements_to_hocr_words(&elements, 0.6);

        assert_eq!(words.len(), 1);
        assert_eq!(words[0].text, "word1");
    }

    #[cfg(paddle_ocr)]
    #[test]
    fn elements_to_hocr_words_prefers_words_over_lines_globally() {
        let elements = vec![
            table_input_element("word one", OcrElementLevel::Word, 0.91),
            table_input_element("full line", OcrElementLevel::Line, 0.99),
            table_input_element("word two", OcrElementLevel::Word, 0.92),
        ];

        let words = elements_to_hocr_words(&elements, 0.6);

        assert_eq!(
            words.iter().map(|word| word.text.as_str()).collect::<Vec<_>>(),
            ["word one", "word two"]
        );
    }

    #[cfg(paddle_ocr)]
    #[test]
    fn elements_to_hocr_words_falls_back_to_lines_when_words_are_absent() {
        let elements = vec![
            table_input_element("line one", OcrElementLevel::Line, 0.91),
            table_input_element("line two", OcrElementLevel::Line, 0.92),
        ];

        let words = elements_to_hocr_words(&elements, 0.6);

        assert_eq!(
            words.iter().map(|word| word.text.as_str()).collect::<Vec<_>>(),
            ["line one", "line two"]
        );
    }

    #[cfg(paddle_ocr)]
    #[test]
    fn elements_to_hocr_words_falls_back_to_lines_when_words_fail_confidence() {
        let elements = vec![
            table_input_element("low word", OcrElementLevel::Word, 0.59),
            table_input_element("fallback line", OcrElementLevel::Line, 0.81),
        ];

        let words = elements_to_hocr_words(&elements, 0.6);

        assert_eq!(words.len(), 1);
        assert_eq!(words[0].text, "fallback line");
    }

    #[cfg(paddle_ocr)]
    fn table_input_element(text: &str, level: OcrElementLevel, confidence: f64) -> OcrElement {
        OcrElement::new(
            text,
            OcrBoundingGeometry::Rectangle {
                left: 0,
                top: 0,
                width: 50,
                height: 20,
            },
            OcrConfidence::from_tesseract(confidence * 100.0),
        )
        .with_level(level)
    }

    #[cfg(paddle_ocr)]
    #[test]
    fn test_text_block_to_element() {
        use xberg_paddle_ocr::Point;

        let block = TextBlock {
            box_points: vec![
                Point { x: 10, y: 20 },
                Point { x: 100, y: 22 },
                Point { x: 98, y: 70 },
                Point { x: 8, y: 68 },
            ],
            box_score: 0.95,
            angle_index: 0,
            angle_score: 0.99,
            text: "Test text".to_string(),
            text_score: 0.88,
        };

        let element = text_block_to_element(&block, 1)
            .expect("Valid TextBlock")
            .expect("box_score is high enough");

        assert_eq!(element.text, "Test text");
        assert_eq!(element.level, OcrElementLevel::Line);
        assert_eq!(element.page_number, 1);

        if let OcrBoundingGeometry::Quadrilateral { points } = &element.geometry {
            assert_eq!(points[0], (10, 20).into());
            assert_eq!(points[1], (100, 22).into());
            assert_eq!(points[2], (98, 70).into());
            assert_eq!(points[3], (8, 68).into());
        } else {
            panic!("Expected Quadrilateral geometry");
        }

        assert!(element.confidence.detection.is_some());
        assert!((element.confidence.detection.unwrap() - 0.95).abs() < 0.001);
        assert!((element.confidence.recognition - 0.88).abs() < 0.001);

        assert!(element.rotation.is_some());
        let rot = element.rotation.as_ref().unwrap();
        assert_eq!(rot.angle_degrees, 0.0);
        assert!((rot.confidence.unwrap() - 0.99).abs() < 0.001);
    }

    #[cfg(paddle_ocr)]
    #[test]
    fn detailed_text_block_preserves_line_and_word_geometry_separately() {
        use xberg_paddle_ocr::{Point, RecognizedWord, WordBlock};

        let block = DetailedTextBlock {
            block: TextBlock {
                box_points: vec![
                    Point { x: 10, y: 20 },
                    Point { x: 110, y: 20 },
                    Point { x: 110, y: 40 },
                    Point { x: 10, y: 40 },
                ],
                box_score: 0.95,
                angle_index: 0,
                angle_score: 0.99,
                text: "first second".to_string(),
                text_score: 0.88,
            },
            words: vec![WordBlock {
                word: RecognizedWord {
                    text: "first".to_string(),
                    confidence: 0.91,
                    ..Default::default()
                },
                box_points: vec![
                    Point { x: 10, y: 20 },
                    Point { x: 50, y: 20 },
                    Point { x: 50, y: 40 },
                    Point { x: 10, y: 40 },
                ],
            }],
            line_column_count: 20.0,
            rotation_retained: false,
        };

        let group = detailed_text_block_to_elements(&block, 2)
            .expect("valid detailed block")
            .expect("detection passes confidence threshold");

        assert_eq!(group.line.text, "first second");
        assert_eq!(group.line.level, OcrElementLevel::Line);
        assert_eq!(group.words.len(), 1);
        assert_eq!(group.words[0].text, "first");
        assert_eq!(group.words[0].level, OcrElementLevel::Word);
        assert_eq!(group.words[0].page_number, 2);
        assert!((group.words[0].confidence.recognition - 0.91).abs() < 0.001);
        assert_eq!(
            group.words[0].geometry,
            OcrBoundingGeometry::Quadrilateral {
                points: [(10, 20), (50, 20), (50, 40), (10, 40)]
                    .into_iter()
                    .map(Into::into)
                    .collect()
            }
        );
    }

    fn line_keyed_word(line: usize, text: &str, left: u32, top: u32, recognition: f64) -> (usize, OcrElement) {
        let geometry = OcrBoundingGeometry::Rectangle {
            left,
            top,
            width: 20,
            height: 10,
        };
        let confidence = OcrConfidence {
            detection: None,
            recognition,
        };
        let word = OcrElement::new(text.to_string(), geometry, confidence)
            .with_level(OcrElementLevel::Word)
            .with_page_number(3)
            .with_metadata("backend", serde_json::json!("tesseract-iterator"));
        (line, word)
    }

    #[test]
    fn tesseract_elements_with_lines_adds_one_line_per_text_line() {
        let elements = tesseract_elements_with_lines(vec![
            line_keyed_word(1, "Total", 10, 5, 0.9),
            line_keyed_word(1, "due", 40, 7, 0.6),
            line_keyed_word(2, "42", 10, 30, 0.8),
        ]);

        let (lines, words) = elements.split_at(2);
        assert_eq!(
            lines.iter().map(|line| line.text.as_str()).collect::<Vec<_>>(),
            ["Total due", "42"]
        );
        assert!(
            lines
                .iter()
                .all(|line| line.level == OcrElementLevel::Line && line.page_number == 3)
        );
        assert_eq!(
            lines[0].geometry,
            OcrBoundingGeometry::Rectangle {
                left: 10,
                top: 5,
                width: 50,
                height: 12,
            }
        );
        assert!((lines[0].confidence.recognition - 0.75).abs() < 1e-9);
        assert_eq!(lines[0].confidence.detection, None);
        assert_eq!(
            lines[0].backend_metadata.get("backend"),
            Some(&serde_json::json!("tesseract-iterator"))
        );
        assert_eq!(
            words.iter().map(|word| word.text.as_str()).collect::<Vec<_>>(),
            ["Total", "due", "42"]
        );
        assert!(words.iter().all(|word| word.level == OcrElementLevel::Word));
    }

    #[test]
    fn iterator_word_to_element_forwards_underline_font_id_crown_indent_and_language() {
        let mut word = xberg_tesseract::WordData::default();
        word.text = "Word".to_string();
        word.left = 10;
        word.top = 20;
        word.right = 60;
        word.bottom = 40;
        word.confidence = 91.0;
        word.font_attrs = Some(xberg_tesseract::FontAttributes {
            is_bold: false,
            is_italic: false,
            is_underlined: true,
            is_monospace: false,
            is_serif: false,
            is_smallcaps: false,
            pointsize: 12,
            font_id: 7,
        });
        word.language = Some("deu".to_string());
        let para = xberg_tesseract::ParaInfo {
            justification: xberg_tesseract::TessParagraphJustification::JUSTIFICATION_LEFT,
            is_list_item: false,
            is_crown: true,
            first_line_indent: 24,
            left: 0,
            top: 0,
            right: 100,
            bottom: 100,
        };

        let element = iterator_word_to_element(&word, None, Some(&para), 1);

        assert_eq!(
            element.backend_metadata.get("is_underlined"),
            Some(&serde_json::json!(true))
        );
        assert_eq!(element.backend_metadata.get("font_id"), Some(&serde_json::json!(7)));
        assert_eq!(element.backend_metadata.get("is_crown"), Some(&serde_json::json!(true)));
        assert_eq!(
            element.backend_metadata.get("first_line_indent"),
            Some(&serde_json::json!(24))
        );
        assert_eq!(
            element.backend_metadata.get("word_language"),
            Some(&serde_json::json!("deu"))
        );
    }

    #[test]
    fn iterator_word_to_element_omits_first_line_indent_and_font_id_when_zero_or_negative() {
        let mut word = xberg_tesseract::WordData::default();
        word.text = "Word".to_string();
        word.left = 10;
        word.top = 20;
        word.right = 60;
        word.bottom = 40;
        word.confidence = 91.0;
        word.font_attrs = Some(xberg_tesseract::FontAttributes {
            is_bold: false,
            is_italic: false,
            is_underlined: false,
            is_monospace: false,
            is_serif: false,
            is_smallcaps: false,
            pointsize: 12,
            font_id: -1,
        });
        let para = xberg_tesseract::ParaInfo {
            justification: xberg_tesseract::TessParagraphJustification::JUSTIFICATION_UNKNOWN,
            is_list_item: false,
            is_crown: false,
            first_line_indent: 0,
            left: 0,
            top: 0,
            right: 100,
            bottom: 100,
        };

        let element = iterator_word_to_element(&word, None, Some(&para), 1);

        assert!(!element.backend_metadata.contains_key("font_id"));
        assert!(!element.backend_metadata.contains_key("first_line_indent"));
        assert!(!element.backend_metadata.contains_key("word_language"));
        assert_eq!(
            element.backend_metadata.get("is_crown"),
            Some(&serde_json::json!(false))
        );
    }

    #[cfg(paddle_ocr)]
    #[test]
    fn test_text_block_to_element_malformed_box_points() {
        use xberg_paddle_ocr::Point;

        let block = TextBlock {
            box_points: vec![Point { x: 10, y: 20 }, Point { x: 100, y: 22 }],
            box_score: 0.95,
            angle_index: 0,
            angle_score: 0.99,
            text: "Test text".to_string(),
            text_score: 0.88,
        };

        let result = text_block_to_element(&block, 1);
        assert!(result.is_err(), "Should reject TextBlock with < 4 box_points");

        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("box_points"), "Error should mention box_points");
        assert!(error_msg.contains("4"), "Error should mention required count");
    }

    #[cfg(paddle_ocr)]
    #[test]
    fn test_text_block_to_element_invalid_angle_index() {
        use xberg_paddle_ocr::Point;

        let block = TextBlock {
            box_points: vec![
                Point { x: 10, y: 20 },
                Point { x: 100, y: 22 },
                Point { x: 98, y: 70 },
                Point { x: 8, y: 68 },
            ],
            box_score: 0.95,
            angle_index: 5,
            angle_score: 0.99,
            text: "Test text".to_string(),
            text_score: 0.88,
        };

        let result = text_block_to_element(&block, 1);
        assert!(result.is_err(), "Should reject TextBlock with invalid angle_index");

        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("angle_index"), "Error should mention angle_index");
    }

    #[cfg(paddle_ocr)]
    fn line_at(left: f64, top: f64, right: f64, bottom: f64) -> OcrElement {
        let geometry = OcrBoundingGeometry::Rectangle {
            left: left as u32,
            top: top as u32,
            width: (right - left) as u32,
            height: (bottom - top) as u32,
        };
        OcrElement::new("line", geometry, OcrConfidence::from_paddle(0.9, 0.9)).with_level(OcrElementLevel::Line)
    }

    #[cfg(paddle_ocr)]
    #[test]
    fn assign_line_block_ids_joins_close_overlapping_consecutive_lines() {
        // Two 20px-tall lines stacked with a 4px gap (0.2x line height) and full
        // x-overlap: well within tolerance, so they must share a block id (#631).
        let elements = vec![line_at(100.0, 100.0, 500.0, 120.0), line_at(100.0, 124.0, 500.0, 144.0)];

        let block_ids = assign_line_block_ids(&elements);

        assert_eq!(block_ids.len(), 2);
        assert_eq!(
            block_ids[0], block_ids[1],
            "close overlapping lines must share a block id"
        );
    }

    #[cfg(paddle_ocr)]
    #[test]
    fn assign_line_block_ids_splits_on_large_vertical_gap() {
        // Same lines as above, but separated by a 200px gap (10x line height):
        // clearly a new paragraph, so the block id must change (#631).
        let elements = vec![line_at(100.0, 100.0, 500.0, 120.0), line_at(100.0, 320.0, 500.0, 340.0)];

        let block_ids = assign_line_block_ids(&elements);

        assert_ne!(
            block_ids[0], block_ids[1],
            "a gap far exceeding line height must start a new block"
        );
    }

    #[cfg(paddle_ocr)]
    #[test]
    fn assign_line_block_ids_grouping_is_scale_invariant_across_dpi() {
        // The same page geometry rendered at 3x DPI (all coordinates scaled
        // uniformly) must produce an identical grouping decision, because the
        // gap/overlap tolerances are relative to the running median line height,
        // never an absolute pixel count (#631).
        let base = vec![
            line_at(100.0, 100.0, 500.0, 120.0),
            line_at(100.0, 124.0, 500.0, 144.0),
            line_at(100.0, 400.0, 500.0, 420.0),
        ];
        let scaled = base
            .iter()
            .map(|element| {
                let (left, top, width, height) = element.geometry.to_aabb();
                const SCALE: u32 = 3;
                line_at(
                    f64::from(left * SCALE),
                    f64::from(top * SCALE),
                    f64::from((left + width) * SCALE),
                    f64::from((top + height) * SCALE),
                )
            })
            .collect::<Vec<_>>();

        let base_ids = assign_line_block_ids(&base);
        let scaled_ids = assign_line_block_ids(&scaled);

        let base_pattern = [base_ids[0] == base_ids[1], base_ids[1] == base_ids[2]];
        let scaled_pattern = [scaled_ids[0] == scaled_ids[1], scaled_ids[1] == scaled_ids[2]];
        assert_eq!(
            base_pattern, scaled_pattern,
            "grouping must be identical under uniform DPI scaling"
        );
    }

    /// Regression test for the "16 pages -> 16 paragraphs" defect: a realistic
    /// 300 DPI scanned page of single-spaced body text where paragraph breaks
    /// are marked ONLY by first-line indentation (left margin 200px, indent
    /// 300px), never by extra vertical whitespace — every line, including the
    /// first line of a new paragraph, sits exactly 16px below the previous
    /// line's bottom edge (well under the 0.6x-line-height gap threshold).
    ///
    /// Before the indent check, `lines_share_block` only looks at vertical
    /// gap and horizontal overlap, both of which pass for every consecutive
    /// pair here, so the whole page collapses into ONE block — reproducing
    /// paddle-ocr emitting a single fused paragraph per page. Against the
    /// unfixed function this assertion fails: all eight entries come back as
    /// `"paddle-block-1"` instead of splitting into three blocks.
    #[cfg(paddle_ocr)]
    #[test]
    fn assign_line_block_ids_splits_indent_marked_paragraphs_with_uniform_leading() {
        let elements = vec![
            // Paragraph 1: indented first line, two flush continuation lines.
            line_at(300.0, 200.0, 2200.0, 234.0),
            line_at(200.0, 250.0, 2200.0, 284.0),
            line_at(200.0, 300.0, 1000.0, 334.0),
            // Paragraph 2: indented first line (same 16px gap as within para 1).
            line_at(300.0, 350.0, 2200.0, 384.0),
            line_at(200.0, 400.0, 2200.0, 434.0),
            line_at(200.0, 450.0, 700.0, 484.0),
            // Paragraph 3: indented first line (same 16px gap again).
            line_at(300.0, 500.0, 2200.0, 534.0),
            line_at(200.0, 550.0, 2200.0, 584.0),
        ];

        let block_ids = assign_line_block_ids(&elements);

        assert_eq!(
            block_ids,
            vec![
                "paddle-block-1",
                "paddle-block-1",
                "paddle-block-1",
                "paddle-block-2",
                "paddle-block-2",
                "paddle-block-2",
                "paddle-block-3",
                "paddle-block-3",
            ],
            "indented first lines must start a new block even though the vertical gap never changes"
        );
    }

    /// Companion to the indent-split test: continuation lines that are flush
    /// with the block's established left margin must NOT be split apart just
    /// because their left edge differs slightly from the immediately
    /// preceding line (OCR boxes are never pixel-perfect).
    #[cfg(paddle_ocr)]
    #[test]
    fn assign_line_block_ids_does_not_split_flush_continuation_lines() {
        let elements = vec![
            line_at(300.0, 200.0, 2200.0, 234.0),
            line_at(201.0, 250.0, 2200.0, 284.0),
            line_at(199.0, 300.0, 2200.0, 334.0),
        ];

        let block_ids = assign_line_block_ids(&elements);

        assert_eq!(
            block_ids,
            vec!["paddle-block-1", "paddle-block-1", "paddle-block-1"],
            "sub-pixel left-edge jitter on flush continuation lines must not start a new block"
        );
    }
}
