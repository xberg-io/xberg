use super::*;
use crate::pdf::native::hierarchy::script_ownership::page_segments::NativePageSegments;
use crate::pdf::native::hierarchy::{ArtifactRetention, ScriptGeometry};
use xberg_native_pdf::geometry::Rect;
use xberg_native_pdf::layout::TextSpan;
use xberg_native_pdf::structure::McidScope;

fn span(text: &str, x: f32, y: f32, width: f32, font_size: f32) -> TextSpan {
    TextSpan {
        text: text.to_owned(),
        bbox: Rect::new(x, y, width, font_size),
        font_size,
        font_name: "source-projection-font".to_owned(),
        mcid_scope: Some(McidScope::Page(0)),
        ..Default::default()
    }
}

fn paragraph(spans: Vec<TextSpan>) -> PdfParagraph {
    let native = NativePageSegments::from_final_spans(
        spans,
        &Default::default(),
        ArtifactRetention::default(),
        ScriptGeometry::Preserve,
    );
    let projected = native.segments().cloned().collect();
    let (segments, evidence) = super::super::native_scripts::prepare_segments(projected, native, &[]);
    let evidence = evidence.expect("real native source carrier");
    assert_eq!(evidence.owners.len(), 3, "complete three-member native power");
    let lines = super::super::native_scripts::reconstruct_native_lines(&segments.iter().collect::<Vec<_>>(), &evidence);
    PdfParagraph {
        text: String::new(),
        word_count: PdfParagraph::compute_word_count("", &lines),
        lines,
        dominant_font_size: 12.0,
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

#[test]
fn should_preserve_canonical_rendering_and_utf8_source_ranges() {
    let paragraph = paragraph(vec![
        span("é reads the follow-", 42.0, 700.0, 200.0, 12.0),
        span("ing source power x", 42.0, 680.8, 140.0, 12.0),
        span("1", 182.0, 685.7568, 4.0, 8.0),
        span("/", 186.0, 685.7568, 3.0, 8.0),
        span("3", 189.0, 685.7568, 4.0, 8.0),
        span(" remains exact.", 193.0, 680.8, 110.0, 12.0),
    ]);
    assert_eq!(paragraph.lines.len(), 2, "actual separate source baselines");
    assert_eq!(
        paragraph
            .lines
            .iter()
            .map(|line| line.valid_native_scripts().len())
            .sum::<usize>(),
        3
    );
    let witnesses = Default::default();
    let ordinary = extract_text_and_annotations(&paragraph, &witnesses);
    let projected = extract_text_with_source_ranges(&paragraph, &witnesses);
    assert_eq!(projected.text, "é reads the following source power x1/3 remains exact.");
    assert_eq!((&projected.text, &projected.annotations), (&ordinary.0, &ordinary.1));
    assert_eq!(projected.segment_ranges.len(), 6, "every actual source segment visited");
    let slices: Vec<_> = projected
        .segment_ranges
        .iter()
        .map(|range| projected.text.get(range.clone()).expect("UTF-8 byte boundaries"))
        .collect();
    assert_eq!(
        slices,
        [
            "é reads the follow",
            "ing source power x",
            "1",
            "/",
            "3",
            "remains exact."
        ]
    );
    assert_eq!(projected.segment_ranges[0].end, "é reads the follow".len());
    assert!(projected.segment_ranges[0].end > "é reads the follow".chars().count());
    let powers: Vec<_> = projected
        .annotations
        .iter()
        .filter(|annotation| annotation.kind == crate::types::document_structure::AnnotationKind::Superscript)
        .collect();
    assert_eq!(powers.len(), 1);
    assert_eq!(&projected.text[powers[0].start as usize..powers[0].end as usize], "1/3");
}

#[test]
fn should_retain_native_source_when_a_list_cut_crosses_a_source_record() {
    let source = paragraph(vec![
        span(
            "The source has a genuine list: to retain the source power x",
            42.0,
            700.0,
            300.0,
            12.0,
        ),
        span("1", 342.0, 704.9568, 4.0, 8.0),
        span("/", 346.0, 704.9568, 3.0, 8.0),
        span("3", 349.0, 704.9568, 4.0, 8.0),
        span(";", 353.0, 700.0, 4.0, 12.0),
        span("where the ordinary sentence continues.", 42.0, 680.8, 250.0, 12.0),
    ]);
    let witnesses = Default::default();
    let before = extract_text_and_annotations(&source, &witnesses);
    let mut pages = vec![vec![source]];
    split_with_witnesses(&mut pages, &witnesses);
    assert_eq!(
        pages[0].len(),
        1,
        "an intra-span cut cannot discard or duplicate its native witness"
    );
    assert_eq!(extract_text_and_annotations(&pages[0][0], &witnesses), before);
    assert_eq!(
        pages[0][0].lines.iter().map(|line| line.segments.len()).sum::<usize>(),
        6
    );
    assert_eq!(
        pages[0][0]
            .lines
            .iter()
            .map(|line| line.valid_native_scripts().len())
            .sum::<usize>(),
        3
    );
}
