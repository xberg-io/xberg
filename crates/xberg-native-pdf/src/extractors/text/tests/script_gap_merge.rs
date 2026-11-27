//! Adjacent-span merge across a sub/superscript drawn in the gap between two base-line runs. ~keep

use super::super::*;

fn base_line_run(text: &str, x: f32, width: f32) -> TextSpan {
    TextSpan {
        text: text.to_string(),
        bbox: Rect::new(x, 700.0, width, 12.0),
        font_name: "F1".into(),
        font_size: 12.0,
        ..TextSpan::default()
    }
}

/// `H₂SO₄` after the reading-order sort: the lowered `2` is not snapped onto the base line, so
/// it sorts after it and the fold sees `H` and `SO` side by side across the subscript's gap.
fn h2so4_runs(with_subscript: bool) -> Vec<TextSpan> {
    let mut runs = vec![base_line_run("0.5 M H", 100.0, 30.0), base_line_run("SO", 134.5, 14.0)];
    if with_subscript {
        runs.push(TextSpan {
            text: "2".to_string(),
            bbox: Rect::new(130.0, 698.0, 4.0, 8.0),
            font_name: "F1".into(),
            font_size: 8.0,
            ..TextSpan::default()
        });
    }
    runs
}

#[test]
fn test_merge_adjacent_spans_keeps_runs_apart_across_a_subscript_in_the_gap() {
    let mut control = TextExtractor::new();
    control.merging_config = SpanMergingConfig::legacy();
    control.spans = h2so4_runs(false);
    control.merge_adjacent_spans();
    assert_eq!(control.spans.len(), 1, "an empty gap of this width merges the runs");

    let mut extractor = TextExtractor::new();
    extractor.merging_config = SpanMergingConfig::legacy();
    extractor.spans = h2so4_runs(true);
    extractor.merge_adjacent_spans();
    let texts: Vec<&str> = extractor.spans.iter().map(|span| span.text.as_str()).collect();
    assert_eq!(texts, ["0.5 M H", "SO", "2"]);
}

#[test]
fn test_merge_adjacent_spans_ignores_a_full_size_neighbour_line_clipping_the_gap() {
    let mut extractor = TextExtractor::new();
    extractor.merging_config = SpanMergingConfig::legacy();
    let mut runs = h2so4_runs(false);
    runs.push(TextSpan {
        text: "below".to_string(),
        bbox: Rect::new(120.0, 689.0, 40.0, 12.0),
        font_name: "F1".into(),
        font_size: 12.0,
        ..TextSpan::default()
    });
    extractor.spans = runs;
    extractor.merge_adjacent_spans();
    assert_eq!(extractor.spans.len(), 2, "a neighbour line is not a script in the gap");
    assert!(extractor.spans[0].text.starts_with("0.5 M H") && extractor.spans[0].text.ends_with("SO"));
    assert_eq!(extractor.spans[1].text, "below");
}

fn hostile_run(text: &str, x: f32, y: f32, height: f32) -> TextSpan {
    TextSpan {
        text: text.to_string(),
        bbox: Rect::new(x, y, 10.0, height),
        font_name: "F1".into(),
        font_size: height,
        ..TextSpan::default()
    }
}

/// `BT /F1 1e9 Tf (x) Tj ET` and a `1e30` run once bucketed every 4 pt of their height: 2.5e8
/// boxes for the first, and a saturated `i64::MIN..=i64::MAX` loop for the second.
#[test]
fn test_merge_adjacent_spans_finishes_with_hostile_font_sizes() {
    let mut extractor = TextExtractor::new();
    extractor.merging_config = SpanMergingConfig::legacy();
    let mut runs = h2so4_runs(false);
    runs.push(hostile_run("x", 400.0, 0.0, 1.0e9));
    runs.push(hostile_run("y", 450.0, -1.0e30, 2.0e30));
    extractor.spans = runs;
    extractor.merge_adjacent_spans();
    let texts: Vec<&str> = extractor.spans.iter().map(|span| span.text.as_str()).collect();
    assert!(texts[0].starts_with("0.5 M H") && texts[0].ends_with("SO"), "{texts:?}");
    let joined = texts.concat();
    assert!(joined.contains('x') && joined.contains('y'), "{texts:?}");
}

/// A pair as tall as a hostile run queries a saturated bucket band, which now scans the stored
/// boxes once instead of looping over the band.
#[test]
fn test_merge_adjacent_spans_finishes_a_gap_query_between_hostile_size_runs() {
    let mut extractor = TextExtractor::new();
    extractor.merging_config = SpanMergingConfig::legacy();
    extractor.spans = vec![
        TextSpan {
            text: "H".to_string(),
            bbox: Rect::new(100.0, -1.0e30, 30.0, 2.0e30),
            font_name: "F1".into(),
            font_size: 2.0e30,
            ..TextSpan::default()
        },
        TextSpan {
            text: "SO".to_string(),
            bbox: Rect::new(134.5, -1.0e30, 14.0, 2.0e30),
            font_name: "F1".into(),
            font_size: 2.0e30,
            ..TextSpan::default()
        },
        TextSpan {
            text: "2".to_string(),
            bbox: Rect::new(130.0, 698.0, 4.0, 8.0),
            font_name: "F1".into(),
            font_size: 8.0,
            ..TextSpan::default()
        },
    ];
    extractor.merge_adjacent_spans();
    let texts: Vec<&str> = extractor.spans.iter().map(|span| span.text.as_str()).collect();
    assert!(texts.concat().contains('2'), "{texts:?}");
}
