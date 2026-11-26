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
