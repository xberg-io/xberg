//! Synthetic geometry for the two header shapes that lost their roles: a leading band whose cells
//! wrap over several lines (xberg-io/xberg#3346), and a spanning title over a leaf band
//! (xberg-io/xberg#3347).

use super::*;
use crate::elements::PathContent;
use crate::layout::text_block::FontWeight;

const WRAPPED_COLUMNS: usize = 6;
const WRAPPED_WIDTH: f32 = 40.0;
const BODY_ROWS: usize = 4;

/// A ruled table whose leading band is four lines tall while every body band holds one line:
/// columns 1 and 3 wrap their labels, columns 0 and 5 label only the last line.
fn wrapped_header_grid() -> (Vec<PathContent>, Vec<TextSpan>) {
    let extent = WRAPPED_COLUMNS as f32 * WRAPPED_WIDTH;
    let mut lines = Vec::new();
    for y in [80.0, 40.0, 30.0, 20.0, 10.0, 0.0] {
        lines.push(PathContent::line(0.0, y, extent, y));
    }
    for column in 0..=WRAPPED_COLUMNS {
        lines.push(PathContent::line(
            column as f32 * WRAPPED_WIDTH,
            0.0,
            column as f32 * WRAPPED_WIDTH,
            80.0,
        ));
    }
    let mut spans = Vec::new();
    let mut label = |text: &str, column: usize, y: f32| {
        spans.push(create_test_span(
            text,
            column as f32 * WRAPPED_WIDTH + 1.0,
            y + 1.0,
            WRAPPED_WIDTH - 2.0,
            8.0,
        ));
    };
    label("Amount", 0, 40.0);
    for (text, y) in [
        ("Proposed", 72.0),
        ("Chicago,", 62.0),
        ("Commonwealth", 52.0),
        ("Label D", 42.0),
    ] {
        label(text, 1, y);
    }
    for (text, y) in [("Current", 62.0), ("Rate", 52.0)] {
        label(text, 3, y);
    }
    label("% Diff", 5, 40.0);
    for row in 0..BODY_ROWS {
        let y = (BODY_ROWS - row - 1) as f32 * 10.0;
        for (column, text) in [(0, "$1,000"), (1, "$10.00"), (3, "$9.00"), (5, "20%")] {
            label(text, column, y);
        }
    }
    (lines, spans)
}

#[test]
fn a_wrapped_leading_band_is_one_header_row() {
    let (lines, spans) = wrapped_header_grid();
    let tables = detected(&lines, &spans);
    assert_eq!(tables.len(), 1, "one ruled table");
    let table = &tables[0];
    assert!(table.has_header, "the wrapped band is the header");
    assert_eq!(table.rows.len(), BODY_ROWS + 1, "one header row plus the body");
    assert!(table.rows[0].is_header);
    assert!(table.rows[0].cells.iter().all(|cell| cell.is_header));
    assert_eq!(table.rows[0].cells[1].text, "Proposed\nChicago,\nCommonwealth\nLabel D");
    assert_eq!(table.rows[0].cells[0].text, "Amount");
    assert_eq!(table.rows[0].cells[5].text, "% Diff");
    assert!(table.rows[1..].iter().all(|row| !row.is_header));
    assert_eq!(table.rows[1].cells[0].text, "$1,000");
}

const STACKED_COLUMNS: usize = 5;
const STACKED_WIDTH: f32 = 60.0;

/// A black rule painted as a filled bar, the way the record's producer draws every rule.
fn filled_rule(left: f32, right: f32, y: f32) -> PathContent {
    let mut rule = PathContent::rect(left, y - 0.5, right - left, 1.0);
    rule.stroke_color = None;
    rule.fill_color = Some(Color::black());
    rule
}

/// One spanning title cell after an empty corner cell, over a bold leaf band naming the columns.
fn stacked_header_grid() -> (Vec<PathContent>, Vec<TextSpan>) {
    let extent = STACKED_COLUMNS as f32 * STACKED_WIDTH;
    let mut lines = Vec::new();
    for y in [60.0, 45.0, 30.0, 15.0, 0.0] {
        lines.push(filled_rule(0.0, extent, y));
    }
    lines.push(PathContent::line(0.0, 0.0, 0.0, 60.0));
    lines.push(PathContent::line(60.0, 0.0, 60.0, 60.0));
    for column in [120.0, 180.0, 240.0] {
        lines.push(PathContent::line(column, 0.0, column, 45.0));
    }
    lines.push(PathContent::line(extent, 0.0, extent, 60.0));

    let mut spans = Vec::new();
    let mut styled = |text: &str, x: f32, y: f32, width: f32, bold: bool| {
        let mut span = create_test_span(text, x + 1.0, y + 1.0, width - 2.0, 13.0);
        if bold {
            span.font_weight = FontWeight::Bold;
        }
        spans.push(span);
    };
    styled("Number of Families", 60.0, 45.0, 240.0, true);
    for (column, label) in ["Limit", "1", "2", "3", "4"].iter().enumerate() {
        styled(label, column as f32 * STACKED_WIDTH, 30.0, STACKED_WIDTH, true);
    }
    styled("$100,000", 0.0, 15.0, STACKED_WIDTH, true);
    for (column, text) in ["$200,000", "$11.00", "$19.00", "$24.00", "$29.00"].iter().enumerate() {
        styled(text, column as f32 * STACKED_WIDTH, 0.0, STACKED_WIDTH, column == 0);
    }
    (lines, spans)
}

#[test]
fn a_stacked_title_keeps_its_leaf_band_as_header() {
    let (lines, spans) = stacked_header_grid();
    let tables = detected(&lines, &spans);
    assert_eq!(tables.len(), 1, "one ruled table");
    let table = &tables[0];
    assert!(table.has_header, "the stacked title proves a header");
    let rows = &table.rows;
    assert_eq!(rows.len(), 4, "title, leaves, two body rows");
    assert_eq!(rows[0].cells.len(), 2, "empty corner and the spanning title");
    assert_eq!((rows[0].cells[1].colspan, rows[0].cells[1].rowspan), (4, 1));
    assert_eq!(rows[0].cells[1].text, "Number of Families");
    assert_eq!(
        rows.iter().map(|row| row.is_header).collect::<Vec<_>>(),
        [true, true, false, false]
    );
    assert!(rows[..2].iter().all(|row| row.cells.iter().all(|cell| cell.is_header)));
    assert_eq!(
        rows[1].cells.iter().map(|cell| cell.text.as_str()).collect::<Vec<_>>(),
        ["Limit", "1", "2", "3", "4"]
    );
    assert_eq!(rows[2].cells[0].text, "$100,000");
}
