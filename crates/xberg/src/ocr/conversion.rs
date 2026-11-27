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
#[cfg(any(feature = "ocr", feature = "ocr-wasm"))]
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
        element = element.with_metadata("block_type", serde_json::json!(tesseract_block_type_name(bt)));
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

#[cfg(any(feature = "ocr", feature = "ocr-wasm"))]
fn tesseract_block_type_name(block_type: xberg_tesseract::TessPolyBlockType) -> &'static str {
    match block_type {
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
    }
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
    let mut text = String::new();
    let mut previous: Option<&OcrElement> = None;
    for (_, word) in words {
        if let Some(previous) = previous {
            let gap = crate::table_core::word_spacing::box_gap(previous.geometry.to_aabb(), word.geometry.to_aabb());
            if !crate::table_core::word_spacing::joins_without_space(&previous.text, &word.text, Some(gap)) {
                text.push(' ');
            }
        }
        text.push_str(word.text.trim());
        previous = Some(word);
    }
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
mod tests;
