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
    let mut unlocated = paragraph("unlocated ០", "");
    unlocated.lines.clear();
    let mut paragraphs = [unlocated, paragraph("VOU", "VOU")];
    let before = super::super::document::ocr_paragraphs_plain_text(&paragraphs);
    assert!(!insert(vec![number("០", 30.0)], &mut paragraphs, (1.0, 1.0), 100));
    assert_eq!(super::super::document::ocr_paragraphs_plain_text(&paragraphs), before);
}

#[test]
fn should_not_invent_an_owner_when_two_ocr_lines_overlap() {
    for text in ["VOU", "Amount O due", "០"] {
        assert_overlapping_owner_is_refused(paragraph(text, text));
    }
    for font_size in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        let mut first = paragraph("VOU", "VOU");
        first.lines[0].dominant_font_size = font_size;
        assert_overlapping_owner_is_refused(first);
    }
    let mut first = paragraph("right left", "right");
    first.lines[0].segments[0].x = 80.0;
    first.lines[0].segments.push(segment("left"));
    assert_overlapping_owner_is_refused(first);
}

fn assert_overlapping_owner_is_refused(first: PdfParagraph) {
    let other = paragraph("VOU", "VOU");
    for mut paragraphs in [[first.clone(), other.clone()], [other, first]] {
        let before = super::super::document::ocr_paragraphs_plain_text(&paragraphs);
        assert!(!insert(vec![number("០", 30.0)], &mut paragraphs, (1.0, 1.0), 100));
        assert_eq!(super::super::document::ocr_paragraphs_plain_text(&paragraphs), before);
    }
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
fn should_keep_original_prose_when_numeric_order_is_not_proved() {
    let mut paragraphs = [paragraph("right left", "right")];
    paragraphs[0].lines[0].segments[0].x = 80.0;
    let mut left = segment("left");
    left.width = 40.0;
    paragraphs[0].lines[0].segments.push(left);
    assert!(!insert(vec![number("០", 60.0)], &mut paragraphs, (1.0, 1.0), 100));
    assert_eq!(paragraphs[0].text, "right left");
    assert_eq!(paragraphs[0].lines[0].segments[0].text, "right");
    let mut merged = [paragraph("Amount O due", "Amount O due")];
    assert!(!insert(vec![number("០", 30.0)], &mut merged, (1.0, 1.0), 100));
    assert_eq!(merged[0].text, "Amount O due");
    assert_eq!(merged[0].lines[0].segments[0].text, "Amount O due");
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
        added.font_size, 20.0,
        "the owning line supplies the structural scale, not the first neighbor"
    );
    assert!(added.assigned_role.is_none());
    assert_number_keeps_heading_structure();
    assert_invalid_owner_font_is_refused();
}

fn heading_control_paragraphs() -> Vec<PdfParagraph> {
    (0..8)
        .map(|index| {
            let text = if index == 0 {
                "SECTION TITLE"
            } else {
                "Ordinary descriptive prose remains readable as the body paragraph."
            };
            let mut value = paragraph(text, text);
            let font_size = if index == 0 { 30.0 } else { 20.0 };
            value.dominant_font_size = font_size;
            value.lines[0].dominant_font_size = font_size;
            let baseline = 500.0 - index as f32 * 40.0;
            value.lines[0].baseline_y = baseline;
            let segment = &mut value.lines[0].segments[0];
            segment.y = baseline;
            segment.baseline_y = baseline;
            segment.font_size = font_size;
            value
        })
        .collect()
}

fn heading_signature(paragraphs: Vec<PdfParagraph>) -> Vec<(u8, String)> {
    let config = ExtractionConfig {
        output_format: crate::OutputFormat::Markdown,
        ..Default::default()
    };
    super::super::document::heuristically_restructured_ocr_pages(&[paragraphs], &[600.0], &[], &config)
        .expect("the real document heading pass produces structure")
        .elements
        .into_iter()
        .filter_map(|element| match element.kind {
            crate::types::internal::ElementKind::Heading { level } => Some((level, element.text)),
            _ => None,
        })
        .collect()
}

fn assert_number_keeps_heading_structure() {
    let mut paragraphs = heading_control_paragraphs();
    let line = &mut paragraphs[1].lines[0];
    let mut suffix = line.segments[0].clone();
    line.segments[0].text = "Ordinary".into();
    line.segments[0].width = 20.0;
    suffix.text = "descriptive prose remains readable as the body paragraph.".into();
    suffix.x = 50.0;
    suffix.width = 50.0;
    line.segments.push(suffix);
    let before = heading_signature(paragraphs.clone());
    assert_eq!(before, vec![(1, "SECTION TITLE".into())]);
    let mut numeric = number("០", 30.0);
    numeric.bbox.y = 460.0;
    numeric.baseline = 460.0;
    assert!(insert(vec![numeric], &mut paragraphs, (1.0, 1.0), 100));
    let segments = crate::pdf::structure::adapters::segments_from_ocr_pages(&[paragraphs.clone()]);
    assert!(
        segments[0]
            .iter()
            .all(|segment| segment.font_size.is_finite() && segment.font_size > 0.0)
    );
    assert_eq!(heading_signature(paragraphs), before);
}

fn assert_invalid_owner_font_is_refused() {
    for font_size in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        let mut paragraphs = [paragraph("body", "body")];
        paragraphs[0].lines[0].dominant_font_size = font_size;
        assert!(
            !insert(vec![number("០", 30.0)], &mut paragraphs, (1.0, 1.0), 100),
            "invalid owning-line font scale {font_size}"
        );
        assert_eq!(paragraphs[0].text, "body");
    }
}

#[test]
fn should_keep_adjacent_native_digits_in_one_numeric_candidate() {
    let chars: Vec<TextChar> = [('ប', 0.0), ('១', 20.0), ('២', 40.0), ('ប', 60.0)]
        .into_iter()
        .map(|(character, x)| TextChar::simple(character, Rect::new(x, 0.0, 20.0, 30.0), "Khmer".into(), 30.0))
        .collect();
    let numbers = candidates(&chars);
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
    let (encoded, proof, _) = verifier_fixture();
    let config = ExtractionConfig::default();
    let run = Run::default();
    assert!(
        run.processor.get().is_none(),
        "a document must not initialize an unused verifier"
    );
    let mut first_processor = None;
    for _ in 0..2 {
        let verified = verify_source(
            encoded.clone(),
            &proof,
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

fn verifier_fixture() -> (std::sync::Arc<Vec<u8>>, SourceProof, TextChar) {
    let source = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/pdf_khmer_number_2408/matching.pdf"
    ));
    verifier_fixture_from(source)
}

fn verifier_fixture_from(source: &[u8]) -> (std::sync::Arc<Vec<u8>>, SourceProof, TextChar) {
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
    let numbers = candidates(&document.extract_chars(0).unwrap());
    let shape = shape_proof::prepare(
        &document,
        0,
        &numbers,
        proof_limits(&ExtractionConfig::default()).max_content_size,
    )
    .unwrap()
    .expect("supported real source font");
    (
        std::sync::Arc::new(raster.data),
        SourceProof { media, numbers, shape },
        character,
    )
}

#[test]
fn should_require_proven_page_context_for_numeric_recovery() {
    assert_source_script_loss_is_not_certified_by_a_numeric_crop();
    let matching = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/pdf_khmer_number_2408/matching.pdf"
    ));
    let type0 = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../xberg-native-pdf/tests/fixtures/cid_glyph_raster_2309/custom.pdf"
    ));
    let intact = rendered_integrity(matching);
    let lost = rendered_integrity(type0);
    assert!(!intact.type0_ink_loss, "positive must use a real intact render");
    assert!(lost.type0_ink_loss, "negative must really lose Type0 ink");
    let policy = |raster| PagePolicy {
        page_index_offset: 0,
        accepted: true,
        scale_only: true,
        deferred_structure: false,
        layout: false,
        transformed: false,
        raster,
        structured: false,
        builtin: true,
    };
    assert!(applicable(policy(Some(&intact))), "intact raster remains eligible");
    let mut offset = policy(Some(&intact));
    offset.page_index_offset = 7;
    assert!(
        !applicable(offset),
        "only whole-document local page numbering is qualified"
    );
    let mut deferred = policy(Some(&intact));
    deferred.deferred_structure = true;
    assert!(
        !applicable(deferred),
        "a stage awaiting later document normalization cannot finalize page text"
    );
    assert!(
        !applicable(policy(Some(&lost))),
        "numeric crops cannot certify a Type0 ink-loss page"
    );
    let detached = super::super::pipeline::PageOcrHints::default();
    let raster_dpis = detached.raster_dpis.as_deref().cloned().unwrap_or_default();
    assert!(
        !applicable(policy(raster_dpis.get(&0))),
        "a detached stage without raster integrity must decline recovery"
    );
    let known = super::super::rendering::PageRasterDpis::from_iter([(0, intact)]);
    assert!(
        !applicable(policy(known.get(&1))),
        "another page's evidence cannot certify this page"
    );
}

fn rendered_integrity(source: &[u8]) -> crate::pdf::render::OcrRasterDpi {
    let document = PdfDocument::from_bytes(source.to_vec()).unwrap();
    crate::pdf::render::render_pdf_ocr_page_with_dpi(
        &document,
        0,
        None,
        &crate::extractors::security::SecurityLimits::default(),
    )
    .unwrap()
    .1
}

fn assert_source_script_loss_is_not_certified_by_a_numeric_crop() {
    let native = "ខ្ញុំ\u{FFFD}\u{FFFD} ស្រលាញ់\u{FFFD} ប្រទេស\u{FFFD}\u{FFFD} កម្ពុជា\u{FFFD} សួស្តី\u{FFFD} ភ្នំពេញ";
    let boundary = crate::types::PageBoundary {
        byte_start: 0,
        byte_end: native.len(),
        page_number: 1,
    };
    let dropped = |read: &str| {
        super::super::super::unreadable_script::pages_that_dropped_script(
            &[1],
            native,
            Some(std::slice::from_ref(&boundary)),
            &[read.to_owned()],
        )
    };
    assert!(
        dropped(native).is_empty(),
        "actually retained prose must not request a retry"
    );
    assert_eq!(
        dropped("The office keeps the records"),
        [1],
        "the unaugmented read must really drop Khmer"
    );
    assert_eq!(
        dropped("The office keeps the records ០"),
        [1],
        "a verified numeral must not remove the page from AF's native-script retry"
    );
    for read in ["123", "០១២", "١٢٣", "50% ០١2"] {
        assert!(
            dropped(read).is_empty(),
            "numeric-only reads are not evidence of another prose script: {read}"
        );
    }
}

#[test]
fn should_keep_recovered_page_text_when_final_rendering_has_no_page() {
    use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
    let mut document = InternalDocument::new("pdf");
    let mut reads = vec!["retained ០".to_owned(), "retained ១".to_owned()];
    let before = reads.clone();
    assert!(!synchronize_plain_pages(
        &document,
        &[true, true],
        &mut reads,
        &crate::OutputFormat::Plain
    ));
    assert_eq!(reads, before, "an absent final page must not erase a successful read");
    document.push_element(InternalElement::text(ElementKind::Paragraph, "final ០", 0).with_page(1));
    assert!(synchronize_plain_pages(
        &document,
        &[true, true],
        &mut reads,
        &crate::OutputFormat::Plain
    ));
    assert_eq!(
        reads,
        ["final ០", "retained ១"],
        "render only the page the final document actually supplies"
    );
}

#[test]
fn should_preserve_multiline_authoritative_ocr_text_when_inserting_a_number() {
    let original = "first \tline\nsecond  line";
    let mut value = paragraph(original, "first");
    value.lines[0].segments[0].width = 20.0;
    let mut right = segment("line");
    right.x = 50.0;
    right.width = 30.0;
    value.lines[0].segments.push(right);
    let mut second = value.lines[0].clone();
    second.baseline_y = -40.0;
    second.segments[0].text = "second".into();
    for segment in &mut second.segments {
        segment.y = -40.0;
        segment.baseline_y = -40.0;
    }
    value.lines.push(second);
    let mut paragraphs = [value];
    assert!(insert(vec![number("០", 30.0)], &mut paragraphs, (1.0, 1.0), 100));
    assert_eq!(paragraphs[0].text, "first \t០ line\nsecond  line");
    assert_eq!(
        paragraphs[0].text.replace("០ ", ""),
        original,
        "all pre-existing OCR bytes survive"
    );
    assert_eq!(paragraphs[0].lines[1].segments[0].text, "second");
    assert_eq!(paragraphs[0].word_count, 5);
}

#[test]
fn should_decline_unqualified_primary_languages_before_numeric_recovery() {
    let mut config = ExtractionConfig {
        ocr: Some(crate::OcrConfig::default()),
        ..Default::default()
    };
    assert!(eligible(&config, 0), "the qualified English primary remains supported");
    for language in [vec!["deu"], vec!["khm"], vec!["eng", "khm"], vec!["eng+khm"]] {
        config.ocr.as_mut().unwrap().language = language.iter().map(|value| value.to_string()).collect();
        assert!(!eligible(&config, 0), "unqualified primary language: {language:?}");
    }
}

#[test]
#[ignore = "Run mise run rust:test:khmer-number-recovery with the pinned Khmer model"]
fn should_refuse_when_the_provisioned_crop_model_disappears() {
    if assert_disappearance_with_a_provisioned_fallback() {
        return;
    }
    let directory = tempfile::tempdir().unwrap();
    let model = std::fs::read(std::env::var_os("XBERG_2408_KHM_MODEL").expect("pinned Khmer model")).unwrap();
    let model_path = directory.path().join("khm.traineddata");
    std::fs::write(&model_path, model).unwrap();
    let (encoded, proof, _) = verifier_fixture();
    let config = ExtractionConfig::default();
    let run = Run::default();
    let positive = verify_source(
        encoded.clone(),
        &proof,
        directory.path().to_str().unwrap(),
        &config,
        &run,
    )
    .unwrap();
    assert_eq!(positive[0].text, "០");
    std::fs::remove_file(&model_path).unwrap();
    assert!(
        verify_source(encoded, &proof, directory.path().to_str().unwrap(), &config, &run).is_err(),
        "a missing selected model must not fall back to another root or download a replacement"
    );
    assert!(!model_path.exists());
}

fn assert_disappearance_with_a_provisioned_fallback() -> bool {
    const CHILD: &str = "XBERG_2408_MODEL_DISAPPEARANCE_CHILD";
    if std::env::var_os(CHILD).as_deref() == Some(std::ffi::OsStr::new("1")) {
        return false;
    }
    let fallback = tempfile::tempdir().unwrap();
    std::fs::copy(
        std::env::var_os("XBERG_2408_KHM_MODEL").expect("pinned Khmer model"),
        fallback.path().join("khm.traineddata"),
    )
    .unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .env(CHILD, "1")
        .env("TESSDATA_PREFIX", fallback.path())
        .args([
            "--exact",
            "extractors::pdf::ocr::number_recovery::tests::should_refuse_when_the_provisioned_crop_model_disappears",
            "--include-ignored",
            "--test-threads=1",
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "controlled model fallback child failed:\n{stdout}\n{stderr}"
    );
    assert!(
        stdout.contains("1 passed; 0 failed; 0 ignored"),
        "one child control must execute: {stdout}"
    );
    true
}

#[test]
#[ignore = "Run mise run rust:test:khmer-number-recovery with the pinned Khmer model"]
fn should_refuse_painted_latin_lookalikes_in_a_claimed_khmer_crop() {
    let directory = tempfile::tempdir().unwrap();
    let model = std::fs::read(std::env::var_os("XBERG_2408_KHM_MODEL").expect("pinned model")).unwrap();
    std::fs::write(directory.path().join("khm.traineddata"), model).unwrap();
    std::fs::write(
        directory.path().join("eng.traineddata"),
        xberg_tesseract::bundled_eng_traineddata().unwrap(),
    )
    .unwrap();
    let config = ExtractionConfig::default();
    let run = Run::default();
    let fixtures: [(&str, &[u8]); 3] = [
        (
            "matching",
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/pdf_khmer_number_2408/matching.pdf"
            )),
        ),
        (
            "painted Latin O",
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/pdf_khmer_number_2408/painted_latin_o.pdf"
            )),
        ),
        (
            "painted ASCII 0",
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/pdf_khmer_number_2408/painted_ascii_zero.pdf"
            )),
        ),
    ];
    let mut observed = Vec::new();
    for (name, source) in fixtures {
        let (encoded, proof, character) = verifier_fixture_from(source);
        let media = proof.media;
        let raster = image::DynamicImage::ImageRgb8(image::load_from_memory(&encoded).unwrap().into_rgb8());
        let limits = proof_limits(&config);
        let budget = CropBudget {
            limits: &limits,
            retained: encoded.capacity() + raster.as_bytes().len(),
        };
        let processor = run.processor.get_or_try_init(|| OcrProcessor::new(None)).unwrap();
        for language in ["khm", "eng"] {
            let mut recognition = recognition_config(directory.path().to_str().unwrap(), &config, 150.0);
            recognition.language = language.into();
            let actual = recognize_crop(&raster, media, character.bbox, processor, &mut recognition, &budget)
                .unwrap()
                .unwrap();
            eprintln!("{name}: {language} crop read={:?}", actual.content);
        }
        let verified = verify_source(encoded, &proof, directory.path().to_str().unwrap(), &config, &run).unwrap();
        observed.push((name, verified.len()));
    }
    assert_eq!(
        observed[0],
        ("matching", 1),
        "the genuine source must actually be recovered"
    );
    assert_eq!(
        &observed[1..],
        &[("painted Latin O", 0), ("painted ASCII 0", 0)],
        "different painted glyphs must not inherit the hidden Khmer Unicode claim"
    );
}

#[tokio::test]
#[ignore = "Run mise run rust:test:khmer-number-recovery with the pinned Khmer model"]
async fn should_defer_missing_model_warnings_until_the_primary_page_is_accepted() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(
        directory.path().join("eng.traineddata"),
        xberg_tesseract::bundled_eng_traineddata().unwrap(),
    )
    .unwrap();
    let config = ExtractionConfig {
        ocr: Some(crate::OcrConfig {
            tessdata_path: Some(directory.path().to_owned()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let (_, observe) = prepare_read(config.ocr.clone().unwrap(), &config, 0, Some("ប០ប"));
    assert!(!observe);
    let input = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/pdf_khmer_number_2408/matching.pdf"
    ));
    let document = PdfDocument::from_bytes(input.to_vec()).unwrap();
    let (encoded, _, _) = verifier_fixture();
    let integrity = rendered_integrity(input);
    let metadata = primary_read().metadata;
    let run = std::sync::Arc::new(Run::default());
    let page = || PreparedPage {
        policy: PagePolicy {
            page_index_offset: 0,
            accepted: false,
            scale_only: observe,
            deferred_structure: false,
            layout: false,
            transformed: false,
            raster: Some(&integrity),
            structured: false,
            builtin: true,
        },
        source: Some(PageSource {
            document: &document,
            index: 0,
            layer: "ប០ប",
            raster: encoded.clone(),
            dimensions: (1275, 1650),
            metadata: &metadata,
        }),
        config: &config,
        run: &run,
    };
    exercise_accepted_page_warnings(page, directory.path()).await;
}

async fn exercise_accepted_page_warnings<'a>(page: impl Fn() -> PreparedPage<'a>, directory: &std::path::Path) {
    let mut warnings = Vec::new();
    let mut paragraphs = [paragraph("VOU", "VOU")];
    let mut text = "VOU".to_owned();
    for (accepted, expected) in [(false, 0), (true, 1), (true, 1)] {
        assert!(
            !page()
                .apply(
                    AcceptedRead {
                        accepted,
                        paragraphs: Some(&mut paragraphs),
                        text: &mut text,
                        warnings: &mut warnings
                    },
                    &[],
                    &[]
                )
                .await
        );
        assert_eq!(warnings.len(), expected, "warnings describe accepted pages once");
    }
    let model = std::fs::read(std::env::var_os("XBERG_2408_KHM_MODEL").expect("pinned model")).unwrap();
    std::fs::write(directory.join("khm.traineddata"), model).unwrap();
    for _ in 0..2 {
        assert!(
            !page()
                .apply(
                    AcceptedRead {
                        accepted: true,
                        paragraphs: Some(&mut paragraphs),
                        text: &mut text,
                        warnings: &mut warnings
                    },
                    &[],
                    &[]
                )
                .await
        );
    }
    assert_eq!(
        warnings.len(),
        2,
        "an unbound accepted read reports a distinct, deduplicated proof gap"
    );
    assert!(warnings[1].message.contains("proof limits"));
    assert_eq!(text, "VOU");
}
