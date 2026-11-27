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
fn tesseract_line_joins_adjacent_cjk_words_without_spaces() {
    let elements = tesseract_elements_with_lines(vec![
        line_keyed_word(1, "停车", 10, 5, 0.9),
        line_keyed_word(1, "区", 32, 5, 0.9),
        line_keyed_word(1, "7.5", 54, 5, 0.9),
        line_keyed_word(1, "用地", 76, 5, 0.9),
        line_keyed_word(1, "指标", 200, 5, 0.9),
    ]);

    assert_eq!(elements[0].text, "停车区 7.5 用地 指标");
}

#[test]
fn iterator_word_to_element_forwards_underline_font_id_crown_indent_and_language() {
    let word = xberg_tesseract::WordData {
        text: "Word".to_string(),
        left: 10,
        top: 20,
        right: 60,
        bottom: 40,
        confidence: 91.0,
        font_attrs: Some(xberg_tesseract::FontAttributes {
            is_bold: false,
            is_italic: false,
            is_underlined: true,
            is_monospace: false,
            is_serif: false,
            is_smallcaps: false,
            pointsize: 12,
            font_id: 7,
        }),
        language: Some("deu".to_string()),
    };
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

    let element = iterator_word_to_element(
        &word,
        Some(xberg_tesseract::TessPolyBlockType::PT_HEADING_TEXT),
        Some(&para),
        1,
    );

    assert_eq!(
        element.backend_metadata.get("block_type"),
        Some(&serde_json::json!("PT_HEADING_TEXT"))
    );

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
    let word = xberg_tesseract::WordData {
        text: "Word".to_string(),
        left: 10,
        top: 20,
        right: 60,
        bottom: 40,
        confidence: 91.0,
        font_attrs: Some(xberg_tesseract::FontAttributes {
            is_bold: false,
            is_italic: false,
            is_underlined: false,
            is_monospace: false,
            is_serif: false,
            is_smallcaps: false,
            pointsize: 12,
            font_id: -1,
        }),
        ..Default::default()
    };
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
