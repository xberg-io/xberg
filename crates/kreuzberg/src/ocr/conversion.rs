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
//! use kreuzberg::ocr::conversion::{text_block_to_element, element_to_hocr_word};
//!
//! // Convert PaddleOCR result to unified element (with error handling)
//! let element = text_block_to_element(&text_block)?;
//!
//! // Convert back to HocrWord for table detection
//! let hocr_word = element_to_hocr_word(&element);
//! ```

#[cfg(feature = "paddle-ocr")]
use crate::types::OcrRotation;
use crate::types::{OcrBoundingGeometry, OcrConfidence, OcrElement, OcrElementLevel};

#[cfg(feature = "paddle-ocr")]
use crate::error::{KreuzbergError, Result};

#[cfg(feature = "paddle-ocr")]
use kreuzberg_paddle_ocr::TextBlock;

#[cfg(feature = "paddle-ocr")]
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
#[cfg(feature = "paddle-ocr")]
pub(crate) fn text_block_to_element(block: &TextBlock, page_number: u32) -> Result<Option<OcrElement>> {
    // Filter ghost detections: box_score below 0.3 indicates unreliable detection
    if block.box_score < 0.3 {
        return Ok(None);
    }

    // Validate box_points - PaddleOCR must provide exactly 4 points
    if block.box_points.len() < 4 {
        return Err(KreuzbergError::ocr(format!(
            "PaddleOCR TextBlock has {} box_points, expected at least 4. This indicates malformed OCR output.",
            block.box_points.len()
        )));
    }

    // Convert box_points to quadrilateral format
    // PaddleOCR provides points in clockwise order starting from top-left
    let points: [(u32, u32); 4] = [
        (block.box_points[0].x, block.box_points[0].y),
        (block.box_points[1].x, block.box_points[1].y),
        (block.box_points[2].x, block.box_points[2].y),
        (block.box_points[3].x, block.box_points[3].y),
    ];

    let geometry = OcrBoundingGeometry::Quadrilateral { points };

    let confidence = OcrConfidence::from_paddle(block.box_score, block.text_score);

    // Only include rotation if angle classification was performed
    let rotation = if block.angle_index >= 0 {
        match OcrRotation::from_paddle(block.angle_index, block.angle_score) {
            Ok(rot) => Some(rot),
            Err(msg) => return Err(KreuzbergError::ocr(msg)),
        }
    } else {
        None
    };

    Ok(Some(
        OcrElement::new(block.text.clone(), geometry, confidence)
            .with_level(OcrElementLevel::Line) // PaddleOCR detects lines
            .with_page_number(page_number)
            .with_rotation_opt(rotation)
            .with_metadata("backend", serde_json::json!("paddle-ocr")),
    ))
}

/// Tesseract TSV row data for conversion.
///
/// This struct represents a single row from Tesseract's TSV output format.
/// TSV format includes hierarchical information (block, paragraph, line, word)
/// along with bounding boxes and confidence scores.
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

    // Generate a hierarchical parent ID
    // Tesseract levels: 1=Page, 2=Block, 3=Paragraph, 4=Line, 5=Word
    let parent_id = if row.level == 5 {
        // Word-level: parent is the line
        Some(format!(
            "p{}_b{}_par{}_l{}",
            row.page_num, row.block_num, row.par_num, row.line_num
        ))
    } else if row.level == 4 {
        // Line-level: parent is the paragraph
        Some(format!("p{}_b{}_par{}", row.page_num, row.block_num, row.par_num))
    } else {
        None
    };

    let mut element = OcrElement::new(row.text.clone(), geometry, confidence)
        .with_level(level)
        .with_page_number(row.page_num as u32)
        .with_metadata("backend", serde_json::json!("tesseract"))
        .with_metadata("block_num", serde_json::json!(row.block_num))
        .with_metadata("par_num", serde_json::json!(row.par_num))
        .with_metadata("line_num", serde_json::json!(row.line_num))
        .with_metadata("word_num", serde_json::json!(row.word_num));

    if let Some(pid) = parent_id {
        element = element.with_parent_id(pid);
    }

    element
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
    word: &kreuzberg_tesseract::WordData,
    block_type: Option<kreuzberg_tesseract::TessPolyBlockType>,
    para_info: Option<&kreuzberg_tesseract::ParaInfo>,
    page_number: u32,
) -> OcrElement {
    // Compute width/height from right/bottom - left/top, clamping to zero if inverted.
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

    // Populate font attributes when available.
    if let Some(ref attrs) = word.font_attrs {
        element = element
            .with_metadata("is_bold", serde_json::json!(attrs.is_bold))
            .with_metadata("is_italic", serde_json::json!(attrs.is_italic))
            .with_metadata("is_monospace", serde_json::json!(attrs.is_monospace))
            .with_metadata("is_serif", serde_json::json!(attrs.is_serif))
            .with_metadata("is_smallcaps", serde_json::json!(attrs.is_smallcaps));
        if attrs.pointsize > 0 {
            element = element.with_metadata("pointsize", serde_json::json!(attrs.pointsize));
        }
    }

    // Populate block type when available.
    if let Some(bt) = block_type {
        let block_type_str = match bt {
            kreuzberg_tesseract::TessPolyBlockType::PT_UNKNOWN => "PT_UNKNOWN",
            kreuzberg_tesseract::TessPolyBlockType::PT_FLOWING_TEXT => "PT_FLOWING_TEXT",
            kreuzberg_tesseract::TessPolyBlockType::PT_HEADING_TEXT => "PT_HEADING_TEXT",
            kreuzberg_tesseract::TessPolyBlockType::PT_PULLOUT_TEXT => "PT_PULLOUT_TEXT",
            kreuzberg_tesseract::TessPolyBlockType::PT_EQUATION => "PT_EQUATION",
            kreuzberg_tesseract::TessPolyBlockType::PT_INLINE_EQUATION => "PT_INLINE_EQUATION",
            kreuzberg_tesseract::TessPolyBlockType::PT_TABLE => "PT_TABLE",
            kreuzberg_tesseract::TessPolyBlockType::PT_VERTICAL_TEXT => "PT_VERTICAL_TEXT",
            kreuzberg_tesseract::TessPolyBlockType::PT_CAPTION_TEXT => "PT_CAPTION_TEXT",
            kreuzberg_tesseract::TessPolyBlockType::PT_FLOWING_IMAGE => "PT_FLOWING_IMAGE",
            kreuzberg_tesseract::TessPolyBlockType::PT_HEADING_IMAGE => "PT_HEADING_IMAGE",
            kreuzberg_tesseract::TessPolyBlockType::PT_PULLOUT_IMAGE => "PT_PULLOUT_IMAGE",
            kreuzberg_tesseract::TessPolyBlockType::PT_HORZ_LINE => "PT_HORZ_LINE",
            kreuzberg_tesseract::TessPolyBlockType::PT_VERT_LINE => "PT_VERT_LINE",
            kreuzberg_tesseract::TessPolyBlockType::PT_NOISE => "PT_NOISE",
            kreuzberg_tesseract::TessPolyBlockType::PT_COUNT => "PT_COUNT",
        };
        element = element.with_metadata("block_type", serde_json::json!(block_type_str));
    }

    // Populate paragraph info when available.
    if let Some(para) = para_info {
        let justification_str = match para.justification {
            kreuzberg_tesseract::TessParagraphJustification::JUSTIFICATION_UNKNOWN => "unknown",
            kreuzberg_tesseract::TessParagraphJustification::JUSTIFICATION_LEFT => "left",
            kreuzberg_tesseract::TessParagraphJustification::JUSTIFICATION_CENTER => "center",
            kreuzberg_tesseract::TessParagraphJustification::JUSTIFICATION_RIGHT => "right",
        };
        element = element
            .with_metadata("is_list_item", serde_json::json!(para.is_list_item))
            .with_metadata("justification", serde_json::json!(justification_str));
    }

    element
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
#[cfg(feature = "paddle-ocr")]
pub(crate) fn element_to_hocr_word(element: &OcrElement) -> HocrWord {
    let (left, top, width, height) = element.geometry.to_aabb();

    HocrWord {
        text: element.text.clone(),
        left,
        top,
        width,
        height,
        // HocrWord expects confidence in 0-100 range
        confidence: element.confidence.recognition * 100.0,
    }
}

/// Convert a vector of OcrElements to HocrWords for batch table processing.
///
/// Filters to word-level elements only, as table reconstruction
/// works best with word-level granularity.
///
/// # Arguments
///
/// * `elements` - Slice of OCR elements to convert
/// * `min_confidence` - Minimum recognition confidence threshold (0.0-1.0)
///
/// # Returns
///
/// A vector of HocrWords filtered by confidence and element level.
#[cfg(feature = "paddle-ocr")]
pub(crate) fn elements_to_hocr_words(elements: &[OcrElement], min_confidence: f64) -> Vec<HocrWord> {
    elements
        .iter()
        .filter(|e| e.confidence.recognition >= min_confidence)
        .filter(|e| matches!(e.level, OcrElementLevel::Word | OcrElementLevel::Line))
        .map(element_to_hocr_word)
        .collect()
}

/// Extension trait to add optional rotation to OcrElement builder.
#[cfg(feature = "paddle-ocr")]
trait OcrElementExt {
    fn with_rotation_opt(self, rotation: Option<OcrRotation>) -> Self;
}

#[cfg(feature = "paddle-ocr")]
impl OcrElementExt for OcrElement {
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
            level: 5, // word level (Tesseract: 1=Page, 2=Block, 3=Paragraph, 4=Line, 5=Word)
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
        assert!(element.parent_id.is_some());
        assert_eq!(element.parent_id.as_ref().unwrap(), "p1_b1_par1_l2");

        // Check geometry
        #[cfg(any(feature = "paddle-ocr", feature = "layout-detection"))]
        {
            let (left, top, width, height) = element.geometry.to_aabb();
            assert_eq!((left, top, width, height), (100, 50, 80, 20));
        }

        // Check confidence
        assert!((element.confidence.recognition - 0.95).abs() < 0.001);
        assert!(element.confidence.detection.is_none());
    }

    #[cfg(feature = "paddle-ocr")]
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

    #[cfg(feature = "paddle-ocr")]
    #[test]
    fn test_quadrilateral_to_hocr_word() {
        // Test conversion of rotated quad to AABB
        let geometry = OcrBoundingGeometry::Quadrilateral {
            points: [(10, 22), (108, 20), (110, 72), (12, 74)],
        };
        let confidence = OcrConfidence::from_paddle(0.95, 0.88);
        let element = OcrElement::new("Rotated", geometry, confidence);

        let word = element_to_hocr_word(&element);

        // AABB should be min/max of all points
        assert_eq!(word.left, 10);
        assert_eq!(word.top, 20);
        assert_eq!(word.width, 100); // 110 - 10
        assert_eq!(word.height, 54); // 74 - 20

        // Confidence in 0-100 range
        assert!((word.confidence - 88.0).abs() < 0.1);
    }

    #[cfg(feature = "paddle-ocr")]
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
                OcrConfidence::from_tesseract(50.0), // Low confidence
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
            .with_level(OcrElementLevel::Block), // Block level, should be filtered
        ];

        // Filter with 0.6 confidence threshold
        let words = elements_to_hocr_words(&elements, 0.6);

        // Should only include word1 (word2 is below threshold, block is wrong level)
        assert_eq!(words.len(), 1);
        assert_eq!(words[0].text, "word1");
    }

    #[cfg(feature = "paddle-ocr")]
    #[test]
    fn test_text_block_to_element() {
        use kreuzberg_paddle_ocr::Point;

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

        // Check quadrilateral geometry is preserved
        if let OcrBoundingGeometry::Quadrilateral { points } = &element.geometry {
            assert_eq!(points[0], (10, 20));
            assert_eq!(points[1], (100, 22));
            assert_eq!(points[2], (98, 70));
            assert_eq!(points[3], (8, 68));
        } else {
            panic!("Expected Quadrilateral geometry");
        }

        // Check confidence
        assert!(element.confidence.detection.is_some());
        assert!((element.confidence.detection.unwrap() - 0.95).abs() < 0.001);
        assert!((element.confidence.recognition - 0.88).abs() < 0.001);

        // Check rotation
        assert!(element.rotation.is_some());
        let rot = element.rotation.as_ref().unwrap();
        assert_eq!(rot.angle_degrees, 0.0);
        assert!((rot.confidence.unwrap() - 0.99).abs() < 0.001);
    }

    #[cfg(feature = "paddle-ocr")]
    #[test]
    fn test_text_block_to_element_malformed_box_points() {
        use kreuzberg_paddle_ocr::Point;

        // Test with insufficient box_points
        let block = TextBlock {
            box_points: vec![
                Point { x: 10, y: 20 },
                Point { x: 100, y: 22 },
                // Missing 2 points
            ],
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

    #[cfg(feature = "paddle-ocr")]
    #[test]
    fn test_text_block_to_element_invalid_angle_index() {
        use kreuzberg_paddle_ocr::Point;

        let block = TextBlock {
            box_points: vec![
                Point { x: 10, y: 20 },
                Point { x: 100, y: 22 },
                Point { x: 98, y: 70 },
                Point { x: 8, y: 68 },
            ],
            box_score: 0.95,
            angle_index: 5, // Invalid: must be 0-3
            angle_score: 0.99,
            text: "Test text".to_string(),
            text_score: 0.88,
        };

        let result = text_block_to_element(&block, 1);
        assert!(result.is_err(), "Should reject TextBlock with invalid angle_index");

        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("angle_index"), "Error should mention angle_index");
    }
}
