use super::*;
use crate::pdf::hierarchy::SegmentData;

fn segment(text: &str) -> SegmentData {
    SegmentData {
        text: text.into(),
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 20.0,
        font_size: 20.0,
        is_bold: false,
        is_italic: false,
        is_monospace: false,
        baseline_y: 0.0,
        rotation_degrees: 0.0,
        assigned_role: None,
    }
}

fn paragraph(authoritative: &str, located: &str) -> PdfParagraph {
    let line = PdfLine {
        segments: vec![segment(located)],
        native_scripts: Vec::new(),
        baseline_y: 0.0,
        dominant_font_size: 20.0,
        is_bold: false,
        is_monospace: false,
    };
    PdfParagraph {
        text: authoritative.into(),
        word_count: 1,
        lines: vec![line],
        dominant_font_size: 20.0,
        heading_level: None,
        is_bold: false,
        is_list_item: false,
        is_code_block: false,
        is_formula: false,
        is_page_furniture: false,
        layout_class: None,
        layout_region_path: None,
        caption_for: None,
        block_bbox: None,
    }
}

fn number(text: &str, x: f32) -> Number {
    Number {
        text: text.into(),
        bbox: Rect::new(x, 0.0, 10.0, 20.0),
        baseline: 0.0,
    }
}

#[test]
fn should_keep_authoritative_text_whose_located_segments_are_incomplete() {
    let mut paragraphs = [paragraph("VOU retained tail", "VOU")];
    assert!(!insert(vec![number("០", 30.0)], &mut paragraphs, (1.0, 1.0), 100));
    assert_eq!(paragraphs[0].text, "VOU retained tail");
}

#[test]
fn should_not_invent_an_owner_when_two_ocr_lines_overlap() {
    let mut paragraphs = [paragraph("VOU", "VOU"), paragraph("VOU", "VOU")];
    assert!(!insert(vec![number("០", 30.0)], &mut paragraphs, (1.0, 1.0), 100));
    assert_eq!(
        super::super::document::ocr_paragraphs_plain_text(&paragraphs),
        "VOU\n\nVOU"
    );
}

#[test]
fn should_not_duplicate_a_number_ocr_already_read_on_the_line() {
    let mut paragraphs = [paragraph("០", "០")];
    assert!(!insert(vec![number("០", 30.0)], &mut paragraphs, (1.0, 1.0), 100));
    assert_eq!(paragraphs[0].text, "០");
}

#[test]
fn should_keep_two_independently_verified_occurrences_on_one_line() {
    let mut paragraphs = [paragraph("VOU", "VOU")];
    assert!(insert(
        vec![number("០", 30.0), number("១", 60.0)],
        &mut paragraphs,
        (1.0, 1.0),
        100
    ));
    let output = super::super::document::ocr_paragraphs_plain_text(&paragraphs);
    assert_eq!(output, "VOU ០ ១");
    assert_eq!(paragraphs[0].word_count, 3);
}

#[test]
fn should_refuse_overlapping_claims_for_one_painted_occurrence() {
    let mut paragraphs = [paragraph("VOU", "VOU")];
    let mut shifted = number("០", 30.0);
    shifted.bbox.y += 0.5;
    shifted.baseline += 0.5;
    assert!(!insert(
        vec![number("០", 30.0), shifted],
        &mut paragraphs,
        (1.0, 1.0),
        100
    ));
    assert_eq!(paragraphs[0].text, "VOU");
}

#[test]
fn should_keep_original_prose_with_descending_x_reading_order() {
    let mut paragraphs = [paragraph("right left", "right")];
    paragraphs[0].lines[0].segments[0].x = 80.0;
    let mut left = segment("left");
    left.width = 40.0;
    paragraphs[0].lines[0].segments.push(left);
    assert!(!insert(vec![number("០", 60.0)], &mut paragraphs, (1.0, 1.0), 100));
    assert_eq!(paragraphs[0].text, "right left");
    assert_eq!(paragraphs[0].lines[0].segments[0].text, "right");
}

#[test]
fn should_keep_ocr_junk_but_add_an_unstyled_verified_number() {
    let mut paragraphs = [paragraph("left O right", "left")];
    let line = &mut paragraphs[0].lines[0];
    line.segments[0].width = 20.0;
    line.segments[0].font_size = 99.0;
    line.segments[0].is_bold = true;
    line.segments[0].is_italic = true;
    line.segments[0].is_monospace = true;
    for (text, x, width) in [("O", 30.0, 10.0), ("right", 50.0, 30.0)] {
        let mut word = segment(text);
        word.x = x;
        word.width = width;
        line.segments.push(word);
    }
    assert!(insert(vec![number("០", 30.0)], &mut paragraphs, (1.0, 1.0), 100));
    assert_eq!(
        super::super::document::ocr_paragraphs_plain_text(&paragraphs),
        "left O ០ right"
    );
    let added = &paragraphs[0].lines[0].segments[2];
    assert_eq!(added.text, "០");
    assert_eq!(
        (added.is_bold, added.is_italic, added.is_monospace),
        (false, false, false)
    );
    assert_eq!(
        added.font_size, 0.0,
        "source numeric verification supplies no typography"
    );
    assert!(added.assigned_role.is_none());
}

#[test]
fn should_keep_adjacent_native_digits_in_one_numeric_candidate() {
    let chars = [('ប', 0.0), ('១', 20.0), ('២', 40.0), ('ប', 60.0)]
        .into_iter()
        .map(|(character, x)| TextChar::simple(character, Rect::new(x, 0.0, 20.0, 30.0), "Khmer".into(), 30.0))
        .collect();
    let numbers = candidates(chars);
    assert_eq!(numbers.len(), 1);
    assert_eq!(numbers[0].text, "១២");
    assert_eq!(numbers[0].bbox, Rect::new(20.0, 0.0, 40.0, 30.0));
}

type EligibilityCase = (&'static str, fn(&mut ExtractionConfig), usize, bool);

fn page_policy_cases() -> [EligibilityCase; 7] {
    [
        ("automatic", |_| {}, 0, true),
        ("forced document", |c| c.force_ocr = true, 0, false),
        ("other forced page", |c| c.force_ocr_pages = Some(vec![2]), 0, true),
        ("actual forced page", |c| c.force_ocr_pages = Some(vec![2]), 1, false),
        ("HTML output", |c| c.output_format = crate::OutputFormat::Html, 0, false),
        ("redaction", |c| c.redaction = Some(Default::default()), 0, false),
        (
            "scan strategy",
            |c| c.ocr_strategy = crate::core::config::OcrStrategy::ScannedPages { min_confidence: 0.5 },
            0,
            false,
        ),
    ]
}

fn ocr_override_cases() -> [EligibilityCase; 5] {
    [
        ("missing OCR", |c| c.ocr = None, 0, false),
        (
            "custom backend",
            |c| c.ocr.as_mut().unwrap().backend = "custom".into(),
            0,
            false,
        ),
        (
            "caller Tesseract policy",
            |c| c.ocr.as_mut().unwrap().tesseract_config = Some(Default::default()),
            0,
            false,
        ),
        (
            "caller backend options",
            |c| c.ocr.as_mut().unwrap().backend_options = Some(serde_json::json!({})),
            0,
            false,
        ),
        (
            "explicit pipeline",
            |c| {
                c.ocr.as_mut().unwrap().pipeline =
                    Some(serde_json::from_str(r#"{"stages":[{"backend":"tesseract"}]}"#).unwrap())
            },
            0,
            false,
        ),
    ]
}

#[test]
fn should_honor_the_actual_forced_page_number() {
    for (label, configure, page, expected) in page_policy_cases().into_iter().chain(ocr_override_cases()) {
        let mut config = ExtractionConfig {
            ocr: Some(Default::default()),
            ..Default::default()
        };
        configure(&mut config);
        assert_eq!(eligible(&config, page), expected, "{label}");
    }
}

#[test]
fn should_refuse_crops_outside_the_page_or_live_memory_budget() {
    let raster = image::DynamicImage::new_rgb8(100, 100);
    let media = Rect::new(0.0, 0.0, 100.0, 100.0);
    let valid = Rect::new(20.0, 20.0, 10.0, 10.0);
    for (bounds, budget, expected) in [
        (valid, 0, false),
        (valid, 100_000, true),
        (Rect::new(-1.0, 20.0, 10.0, 10.0), 100_000, false),
        (Rect::new(95.0, 20.0, 10.0, 10.0), 100_000, false),
        (Rect::new(20.0, 95.0, 10.0, 10.0), 100_000, false),
        (Rect::new(20.0, 20.0, 0.0, 10.0), 100_000, false),
        (Rect::new(f32::NAN, 20.0, 10.0, 10.0), 100_000, false),
    ] {
        assert_eq!(
            crop_number(&raster, media, bounds, budget).is_some(),
            expected,
            "{bounds:?}, budget={budget}"
        );
    }
    let large = image::DynamicImage::new_rgb8(1025, 1025);
    let media = Rect::new(0.0, 0.0, 1025.0, 1025.0);
    assert!(crop_number(&large, media, Rect::new(0.0, 0.0, 1024.0, 1024.0), usize::MAX).is_some());
    assert!(
        crop_number(&large, media, media, usize::MAX).is_none(),
        "pixel cap must reject an otherwise valid crop"
    );
}

fn primary_read() -> crate::types::ExtractedDocument {
    use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
    let mut element = InternalElement::text(ElementKind::Paragraph, "VOU", 0);
    element.bbox = Some(crate::types::BoundingBox {
        x0: 10.0,
        y0: 10.0,
        x1: 20.0,
        y1: 20.0,
    });
    let mut doc = InternalDocument::new("ocr");
    doc.elements.push(element);
    let mut read = crate::types::ExtractedDocument {
        content: "VOU".into(),
        ocr_internal_document: Some(doc),
        ..Default::default()
    };
    read.metadata.additional.insert(
        crate::ocr_metadata_keys::OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY.into(),
        100.into(),
    );
    read.metadata.additional.insert(
        crate::ocr_metadata_keys::OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY.into(),
        100.into(),
    );
    read
}

#[test]
fn should_bind_positive_scale_only_evidence_to_the_selected_geometry() {
    let mut read = primary_read();
    assert!(ReadProof::capture(&read, false).is_none());
    let proof = ReadProof::capture(&read, true).expect("observed primary");
    assert!(proof.matches(&read));
    read.ocr_internal_document.as_mut().unwrap().elements[0]
        .bbox
        .as_mut()
        .unwrap()
        .x0 = 11.0;
    assert!(
        !proof.matches(&read),
        "same text with different geometry cannot reuse the observation"
    );
}

#[test]
fn should_refuse_an_oversized_selected_read_proof() {
    let mut read = primary_read();
    read.content = "x".repeat(300 * 1024);
    assert!(ReadProof::capture(&read, true).is_none());
}

#[test]
fn should_normalize_only_spacing_in_the_recognized_numeric_run() {
    for (read, number, expected) in [
        (" ០\u{200b} ១\n", "០១", true),
        ("០\u{00a0}១", "០១", true),
        ("០ ២", "០១", false),
        ("0", "០", false),
        ("០.១", "០១", false),
        ("០letter", "០", false),
        ("\u{200b}\n", "០", false),
    ] {
        assert_eq!(
            recognized_number_matches(read, number),
            expected,
            "read={read:?}, native={number:?}"
        );
    }
}

#[test]
#[ignore = "Run mise run rust:test:khmer-number-recovery with the pinned Khmer model"]
fn should_reuse_one_lazy_processor_for_independent_page_verifications() {
    let directory = tempfile::tempdir().unwrap();
    let model = std::fs::read(std::env::var_os("XBERG_2408_KHM_MODEL").expect("pinned model")).unwrap();
    std::fs::write(directory.path().join("khm.traineddata"), model).unwrap();
    let (encoded, media, character) = verifier_fixture();
    let config = ExtractionConfig::default();
    let run = Run::default();
    assert!(
        run.processor.get().is_none(),
        "a document must not initialize an unused verifier"
    );
    let mut first_processor = None;
    for _ in 0..2 {
        let numbers = vec![Number {
            text: "០".into(),
            bbox: character.bbox,
            baseline: character.origin_y,
        }];
        let verified = verify_source(
            encoded.clone(),
            media,
            numbers,
            directory.path().to_str().unwrap(),
            &config,
            &run,
        )
        .unwrap();
        assert_eq!(
            verified.iter().map(|number| number.text.as_str()).collect::<Vec<_>>(),
            ["០"]
        );
        let processor = run
            .processor
            .get()
            .expect("the real successful crop must use the run's processor");
        if let Some(first) = first_processor {
            assert!(
                std::ptr::eq(first, processor),
                "page verification must retain one native API pool"
            );
        }
        first_processor = Some(processor);
    }
}

fn verifier_fixture() -> (std::sync::Arc<Vec<u8>>, Rect, TextChar) {
    let source = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/pdf_khmer_number_2408/matching.pdf"
    ));
    let document = PdfDocument::from_bytes(source.to_vec()).unwrap();
    let media = document.get_page_info(0).unwrap().media_box;
    let character = document
        .extract_chars(0)
        .unwrap()
        .into_iter()
        .find(|value| value.char == '០')
        .unwrap();
    let raster = xberg_native_pdf::rendering::render_page(
        &document,
        0,
        &xberg_native_pdf::rendering::RenderOptions::with_dpi(150),
    )
    .unwrap();
    (std::sync::Arc::new(raster.data), media, character)
}
