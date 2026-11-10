use super::super::*;
use super::extraction_and_rotation::create_test_font;

const GLYPH_ADVANCE: f32 = 5.0;
const COORDINATE_TOLERANCE: f32 = 0.0001;

fn adaptive_extractor(mode: WordBoundaryMode, offsets: &str) -> TextExtractor<'static> {
    let mut font = create_test_font();
    font.widths = Some((32..=126).map(|code| if code == 32 { 250.0 } else { 500.0 }).collect());
    font.first_char = Some(32);
    font.last_char = Some(126);
    let mut extractor = TextExtractor::with_config(TextExtractionConfig {
        word_boundary_mode: mode,
        use_adaptive_tj_threshold: true,
        ..Default::default()
    });
    extractor.add_font("F1".to_string(), font);
    extractor.extract_spans = true;
    extractor
        .run_content_stream(format!("BT /F1 10 Tf 72 700 Td [{offsets}] TJ").as_bytes())
        .unwrap();
    extractor
        .run_content_stream(b"1 0 0 1 72 700 Tm (abc) Tj 1 0 0 1 80 700 Tm")
        .unwrap();
    extractor
}

fn finish_lead(extractor: &mut TextExtractor<'_>, lead: f32) {
    extractor
        .run_content_stream(format!("[{lead} (def)] TJ ET").as_bytes())
        .unwrap();
    extractor.flush_tj_span_buffer().unwrap();
    extractor.settle_span_holds();
}

fn assert_run(extractor: &TextExtractor<'_>, width: f32, next_x: f32) {
    let abc = extractor.spans.iter().find(|span| span.text == "abc").unwrap();
    let def = extractor.spans.iter().find(|span| span.text == "def").unwrap();
    assert_eq!((abc.bbox.x, abc.bbox.width), (72.0, width));
    assert!((def.bbox.x - next_x).abs() < COORDINATE_TOLERANCE, "{def:?}");
    for span in [abc, def] {
        assert_eq!(span.char_widths.len(), 3);
        assert!(
            span.char_widths
                .iter()
                .all(|width| (width - GLYPH_ADVANCE).abs() < COORDINATE_TOLERANCE)
        );
    }
}

#[test]
fn primary_lead_that_makes_history_justified_keeps_the_kern_pen_width() {
    let mut extractor = adaptive_extractor(WordBoundaryMode::Primary, "-0.1 -0.1");
    assert_eq!(extractor.tj_offset_history, [-0.1, -0.1]);
    assert_eq!(extractor.calculate_adaptive_tj_threshold(), -0.25);
    finish_lead(&mut extractor, -0.5);
    assert_eq!(extractor.tj_offset_history, [-0.1, -0.1, -0.5]);
    assert_eq!(extractor.calculate_adaptive_tj_threshold(), -0.75);
    assert_eq!(extractor.tj_stats_len, 3);
    assert_run(&extractor, 8.0, 80.0);
}

#[test]
fn primary_lead_that_makes_history_nonjustified_restores_the_drawn_width() {
    let mut extractor = adaptive_extractor(WordBoundaryMode::Primary, "-0.1 -0.4");
    assert_eq!(extractor.calculate_adaptive_tj_threshold(), -0.75);
    finish_lead(&mut extractor, -0.3);
    assert_eq!(extractor.tj_offset_history, [-0.1, -0.4, -0.3]);
    assert_eq!(extractor.calculate_adaptive_tj_threshold(), -0.25);
    assert_run(&extractor, 15.0, 80.003);
}

#[test]
fn tiebreaker_keeps_its_prelead_classification_when_history_changes() {
    for (offsets, lead, width, next_x) in [("-0.1 -0.1", -0.5, 15.0, 80.005), ("-0.1 -0.4", -0.3, 8.0, 80.0)] {
        let mut extractor = adaptive_extractor(WordBoundaryMode::Tiebreaker, offsets);
        finish_lead(&mut extractor, lead);
        assert_eq!(extractor.tj_offset_history.len(), 3);
        assert_eq!(extractor.tj_stats_len, 3);
        assert_run(&extractor, width, next_x);
    }
}

#[test]
fn primary_lead_does_not_change_the_capped_history_or_its_classification() {
    let offsets = format!("{}{}", "-0.05 ".repeat(5_000), "-0.14999998 ".repeat(5_000));
    let mut extractor = adaptive_extractor(WordBoundaryMode::Primary, &offsets);
    assert_eq!(extractor.tj_offset_history.len(), 10_000);
    assert_eq!(extractor.calculate_adaptive_tj_threshold(), -0.25);
    let history = extractor.tj_offset_history.clone();
    let stats = (extractor.tj_sum, extractor.tj_sum_sq, extractor.tj_stats_len);
    finish_lead(&mut extractor, -0.5);
    assert_eq!(extractor.tj_offset_history, history);
    assert_eq!((extractor.tj_sum, extractor.tj_sum_sq, extractor.tj_stats_len), stats);
    assert_eq!(extractor.calculate_adaptive_tj_threshold(), -0.25);
    assert_run(&extractor, 15.0, 80.005);
}

#[test]
fn primary_nonempty_fallback_uses_the_same_postrecord_lead_classification() {
    let mut extractor = adaptive_extractor(WordBoundaryMode::Primary, "-0.1 -0.1 (x)");
    assert!(!extractor.tj_character_array.is_empty());
    finish_lead(&mut extractor, -0.5);
    assert_eq!(extractor.tj_offset_history, [-0.1, -0.1, -0.5]);
    assert_eq!(extractor.calculate_adaptive_tj_threshold(), -0.75);
    assert_run(&extractor, 8.0, 80.0);
}

#[test]
fn primary_detected_boundaries_do_not_preview_an_offset_that_will_not_be_recorded() {
    let mut extractor = adaptive_extractor(WordBoundaryMode::Primary, "-0.1 -0.1 (x y)");
    let history = extractor.tj_offset_history.clone();
    let stats = (extractor.tj_sum, extractor.tj_sum_sq, extractor.tj_stats_len);
    finish_lead(&mut extractor, -0.5);
    assert_eq!(extractor.tj_offset_history, history);
    assert_eq!((extractor.tj_sum, extractor.tj_sum_sq, extractor.tj_stats_len), stats);
    let abc = extractor.spans.iter().find(|span| span.text == "abc").unwrap();
    assert_eq!(abc.bbox.width, 15.0);
}
