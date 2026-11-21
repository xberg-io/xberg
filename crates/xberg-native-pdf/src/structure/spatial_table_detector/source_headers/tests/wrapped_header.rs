//! Synthetic geometry for the two header shapes that lost their roles: a leading band whose cells
//! wrap over several lines (xberg-io/xberg#3346), and a spanning title over a leaf band
//! (xberg-io/xberg#3347).

use super::*;
use crate::elements::PathContent;
use crate::layout::text_block::FontWeight;

const COLUMNS: usize = 6;
const WIDTH: f32 = 40.0;
const EXTENT: f32 = COLUMNS as f32 * WIDTH;
const BODY_ROWS: usize = 4;

fn horizontal_rule(left: f32, right: f32, y: f32) -> PathContent {
    PathContent::line(left, y, right, y)
}

/// A rule painted as a filled bar a point thick, the way a producer that never strokes rules draws
/// both grid axes.
fn filled_bar(left: f32, right: f32, y: f32) -> PathContent {
    let mut bar = PathContent::rect(left, y - 0.5, right - left, 1.0);
    bar.stroke_color = None;
    bar.fill_color = Some(Color::black());
    bar
}

fn filled_vertical(x: f32, top: f32, bottom: f32) -> PathContent {
    let mut bar = PathContent::rect(x - 0.5, top, 1.0, bottom - top);
    bar.stroke_color = None;
    bar.fill_color = Some(Color::black());
    bar
}

/// One band of unspanned cells, each column given as its `(text, baseline)` lines.
fn band_row(y: f32, height: f32, columns: &[&[(&str, f32)]]) -> TableRow {
    let cells = columns
        .iter()
        .enumerate()
        .map(|(column, lines)| {
            let left = column as f32 * WIDTH;
            let mut spans = Vec::new();
            for (text, baseline) in *lines {
                spans.push(create_test_span(text, left + 1.0, baseline - 4.0, WIDTH - 2.0, 8.0));
            }
            TableCell {
                text: lines.iter().map(|(text, _)| *text).collect::<Vec<_>>().join("\n"),
                spans,
                colspan: 1,
                rowspan: 1,
                mcids: Vec::new(),
                bbox: Some(Rect::new(left, y, WIDTH, height)),
                is_header: false,
            }
        })
        .collect();
    TableRow {
        cells,
        is_header: false,
    }
}

/// A single-line body band; an empty label leaves its cell unspanned.
fn body_row(y: f32, texts: &[&str]) -> TableRow {
    let columns: Vec<Vec<(&str, f32)>> = texts
        .iter()
        .map(|text| {
            if text.is_empty() {
                Vec::new()
            } else {
                vec![(*text, y + 2.0)]
            }
        })
        .collect();
    let borrowed: Vec<&[(&str, f32)]> = columns.iter().map(Vec::as_slice).collect();
    band_row(y, 10.0, &borrowed)
}

/// The wrapped header band: columns 1 and 3 wrap over four and two lines, columns 0 and 5 label
/// only the last line, and columns 2 and 4 stay empty.
fn wrapped_header_row() -> TableRow {
    band_row(
        40.0,
        40.0,
        &[
            &[("Amount", 42.0)],
            &[
                ("Proposed", 72.0),
                ("Chicago,", 62.0),
                ("Commonwealth", 52.0),
                ("Label D", 42.0),
            ],
            &[],
            &[("Current", 62.0), ("Rate", 52.0)],
            &[],
            &[("% Diff", 42.0)],
        ],
    )
}

fn wrapped_body_rows() -> Vec<TableRow> {
    [30.0, 20.0, 10.0, 0.0]
        .into_iter()
        .map(|y| body_row(y, &["$1,000", "$10.00", "", "$9.00", "", "20%"]))
        .collect()
}

fn wrapped_rows() -> Vec<TableRow> {
    let mut rows = vec![wrapped_header_row()];
    rows.extend(wrapped_body_rows());
    rows
}

/// A form-numbering band the strip removes: one lone digit and five empty cells.
fn numbering_row(y: f32) -> TableRow {
    body_row(y, &["1", "", "", "", "", ""])
}

fn wrapped_rules(bar: bool) -> Vec<PathContent> {
    let rule = |left, right, y| {
        if bar {
            filled_bar(left, right, y)
        } else {
            horizontal_rule(left, right, y)
        }
    };
    // Column rules are painted before row rules, as the producers these fixtures mirror do. ~keep
    let mut rules: Vec<PathContent> = (0..=COLUMNS)
        .map(|column| {
            let x = column as f32 * WIDTH;
            if bar {
                filled_vertical(x, 0.0, 80.0)
            } else {
                PathContent::line(x, 0.0, x, 80.0)
            }
        })
        .collect();
    rules.extend(
        [80.0, 40.0, 30.0, 20.0, 10.0, 0.0]
            .into_iter()
            .map(|y| rule(0.0, EXTENT, y)),
    );
    rules
}

/// A ruled grid whose leading band is four lines tall while every body band holds one line.
fn wrapped_grid(numbering: bool) -> Vec<PathContent> {
    let mut rules = wrapped_rules(false);
    if numbering {
        rules.extend([horizontal_rule(0.0, EXTENT, 90.0), horizontal_rule(0.0, EXTENT, 80.0)]);
        for column in 0..=COLUMNS {
            let x = column as f32 * WIDTH;
            rules.push(PathContent::line(x, 80.0, x, 90.0));
        }
    }
    rules
}

fn grid_spans(rows: &[TableRow]) -> Vec<TextSpan> {
    rows.iter()
        .flat_map(|row| row.cells.iter())
        .flat_map(|cell| cell.spans.clone())
        .collect()
}

fn wrapped_is_header(rows: &mut [TableRow], rules: &[PathContent]) -> bool {
    retain_wrapped_leading_header(rows, rules, 2.0);
    rows.first().is_some_and(|row| row.is_header)
}

#[test]
fn a_wrapped_leading_band_is_one_header_row() {
    let rules = wrapped_rules(false);
    let mut rows = wrapped_rows();
    assert!(wrapped_is_header(&mut rows, &rules));
    assert!(rows[0].cells.iter().all(|cell| cell.is_header));
    assert_eq!(rows[0].cells[1].text, "Proposed Chicago, Commonwealth Label D");
    assert!(
        !rows[0].cells[1].text.contains('\n'),
        "a kept header cell must render on one line"
    );
    assert!(rows[1..].iter().all(|row| !row.is_header));
}

#[test]
fn a_wrapped_leading_band_with_filled_bar_rules_is_one_header_row() {
    let rules = wrapped_rules(true);
    let mut rows = wrapped_rows();
    assert!(wrapped_is_header(&mut rows, &rules));
    assert_eq!(rows[0].cells[0].text, "Amount");
    assert_eq!(rows[0].cells[5].text, "% Diff");
}

#[test]
fn an_underline_inside_one_cell_does_not_veto_the_wrapped_header() {
    let mut rules = wrapped_rules(false);
    // Thirty percent of the band's width, drawn inside column 0 and one column 1: no single
    // column's underline spans the band.
    rules.push(horizontal_rule(0.0, EXTENT * 0.3, 60.0));
    let mut rows = wrapped_rows();
    assert!(wrapped_is_header(&mut rows, &rules));
}

#[test]
fn a_rule_spanning_the_wrapped_band_vetoes_the_header() {
    let mut rules = wrapped_rules(false);
    rules.push(horizontal_rule(0.0, EXTENT, 60.0));
    let mut rows = wrapped_rows();
    assert!(!wrapped_is_header(&mut rows, &rules));
}

#[test]
fn a_missing_lower_edge_rule_vetoes_the_wrapped_header() {
    let rules: Vec<PathContent> = wrapped_rules(false)
        .into_iter()
        .filter(|rule| rule.bbox.center().y != 40.0)
        .collect();
    let mut rows = wrapped_rows();
    assert!(!wrapped_is_header(&mut rows, &rules));
}

#[test]
fn a_data_value_line_inside_the_wrapped_band_vetoes_the_header() {
    let mut rows = wrapped_rows();
    rows[0].cells[3] = band_row(
        40.0,
        40.0,
        &[&[], &[], &[], &[("Current", 62.0), ("$9.00", 52.0)], &[], &[]],
    )
    .cells
    .swap_remove(3);
    assert!(!wrapped_is_header(&mut rows, &wrapped_rules(false)));
}

#[test]
fn a_single_line_leading_band_is_not_a_wrapped_header() {
    let mut rows = wrapped_rows();
    rows[0] = band_row(
        40.0,
        40.0,
        &[
            &[("Amount", 60.0)],
            &[("Proposed", 60.0)],
            &[],
            &[("Current", 60.0)],
            &[],
            &[("% Diff", 60.0)],
        ],
    );
    assert!(!wrapped_is_header(&mut rows, &wrapped_rules(false)));
}

#[test]
fn a_form_numbering_band_above_the_header_is_skipped() {
    let mut rows = vec![numbering_row(80.0), wrapped_header_row()];
    rows.extend(wrapped_body_rows());
    let mut rules = wrapped_rules(false);
    rules.extend([horizontal_rule(0.0, EXTENT, 90.0), horizontal_rule(0.0, EXTENT, 80.0)]);
    retain_wrapped_leading_header(&mut rows, &rules, 2.0);
    assert!(!rows[0].is_header, "the numbering band is not the header");
    assert!(rows[1].is_header, "the wrapped band below it is");
    assert!(rows[1].cells.iter().all(|cell| cell.is_header));
}

const STACKED_COLUMNS: usize = 5;
const STACKED_WIDTH: f32 = 60.0;
const STACKED_EXTENT: f32 = STACKED_COLUMNS as f32 * STACKED_WIDTH;

fn stacked_cell(text: &str, column: usize, colspan: u32, y: f32, width: f32, bold: bool) -> TableCell {
    let left = column as f32 * STACKED_WIDTH;
    let mut spans = Vec::new();
    if !text.is_empty() {
        let mut span = create_test_span(text, left + 1.0, y + 1.0, width - 2.0, 13.0);
        if bold {
            span.font_weight = FontWeight::Bold;
        }
        spans.push(span);
    }
    TableCell {
        text: text.to_string(),
        spans,
        colspan,
        rowspan: 1,
        mcids: Vec::new(),
        bbox: Some(Rect::new(left, y, width, 15.0)),
        is_header: false,
    }
}

fn stacked_title_row(corner: &str) -> TableRow {
    TableRow {
        cells: vec![
            stacked_cell(corner, 0, 1, 45.0, STACKED_WIDTH, true),
            stacked_cell("Number of Families", 1, 4, 45.0, 4.0 * STACKED_WIDTH, true),
        ],
        is_header: false,
    }
}

fn stacked_leaf_row(bold: bool) -> TableRow {
    TableRow {
        cells: ["Limit", "1", "2", "3", "4"]
            .iter()
            .enumerate()
            .map(|(column, label)| stacked_cell(label, column, 1, 30.0, STACKED_WIDTH, bold))
            .collect(),
        is_header: false,
    }
}

fn stacked_body_rows(bold: bool) -> Vec<TableRow> {
    [
        (15.0, ["$100,000", "", "", "", ""]),
        (0.0, ["$200,000", "$11.00", "$19.00", "$24.00", "$29.00"]),
    ]
    .into_iter()
    .map(|(y, labels)| TableRow {
        cells: labels
            .iter()
            .enumerate()
            .map(|(column, label)| stacked_cell(label, column, 1, y, STACKED_WIDTH, bold || column == 0))
            .collect(),
        is_header: false,
    })
    .collect()
}

fn stacked_rules() -> Vec<PathContent> {
    // Column rules are painted before row rules, as the producers these fixtures mirror do. ~keep
    let mut rules = vec![
        filled_vertical(0.0, 0.0, 60.0),
        filled_vertical(STACKED_WIDTH, 0.0, 60.0),
        filled_vertical(120.0, 0.0, 45.0),
        filled_vertical(180.0, 0.0, 45.0),
        filled_vertical(240.0, 0.0, 45.0),
        filled_vertical(STACKED_EXTENT, 0.0, 60.0),
    ];
    rules.extend(
        [60.0, 45.0, 30.0, 15.0, 0.0]
            .into_iter()
            .map(|y| filled_bar(0.0, STACKED_EXTENT, y)),
    );
    rules
}

fn stacked_rows(corner: &str, leaf_bold: bool, body_bold: bool) -> Vec<TableRow> {
    let mut rows = vec![stacked_title_row(corner), stacked_leaf_row(leaf_bold)];
    rows.extend(stacked_body_rows(body_bold));
    rows
}

fn stacked_is_header(rows: &mut [TableRow]) -> bool {
    retain_grouped_leaf_header(rows, &stacked_rules(), STACKED_COLUMNS);
    rows.first().is_some_and(|row| row.is_header)
}

#[test]
fn a_stacked_title_keeps_its_leaf_band_as_header() {
    let mut rows = stacked_rows("", true, false);
    assert!(stacked_is_header(&mut rows));
    assert_eq!(
        rows.iter().map(|row| row.is_header).collect::<Vec<_>>(),
        [true, true, false, false]
    );
    assert!(rows[..2].iter().all(|row| row.cells.iter().all(|cell| cell.is_header)));
}

#[test]
fn a_non_empty_corner_is_not_a_stacked_header() {
    let mut rows = stacked_rows("Corner", true, false);
    assert!(!stacked_is_header(&mut rows));
}

#[test]
fn a_bold_body_band_is_not_a_stacked_header() {
    let mut rows = stacked_rows("", true, true);
    assert!(!stacked_is_header(&mut rows));
}

#[test]
fn a_corner_leaf_without_a_label_is_not_a_stacked_header() {
    let mut rows = stacked_rows("", true, false);
    rows[1] = TableRow {
        cells: ["2024", "Q1", "2", "3", "4"]
            .iter()
            .enumerate()
            .map(|(column, label)| stacked_cell(label, column, 1, 30.0, STACKED_WIDTH, true))
            .collect(),
        is_header: false,
    };
    assert!(!stacked_is_header(&mut rows));
}

#[test]
fn a_non_bold_leaf_cell_is_not_a_stacked_header() {
    let mut rows = stacked_rows("", false, false);
    assert!(!stacked_is_header(&mut rows));
}

#[test]
fn a_wrapped_leading_band_reaches_the_table_as_one_header_row() {
    let rows = wrapped_rows();
    let tables = detected(&wrapped_grid(false), &grid_spans(&rows));
    assert_eq!(tables.len(), 1, "one ruled table");
    let table = &tables[0];
    assert!(table.has_header, "the wrapped band is the header");
    assert_eq!(table.rows.len(), BODY_ROWS + 1, "one header row plus the body");
    assert!(table.rows[0].is_header);
    assert_eq!(table.rows[0].cells[1].text, "Proposed Chicago, Commonwealth Label D");
    assert!(table.rows[0].cells.iter().all(|cell| !cell.text.contains('\n')));
    assert_eq!(table.rows[1].cells[0].text, "$1,000");
}

#[test]
fn a_form_numbering_band_above_the_wrapped_header_is_stripped() {
    let mut rows = vec![numbering_row(80.0)];
    rows.extend(wrapped_rows());
    let tables = detected(&wrapped_grid(true), &grid_spans(&rows));
    assert_eq!(tables.len(), 1, "one ruled table");
    let table = &tables[0];
    assert!(table.has_header);
    assert_eq!(table.rows.len(), BODY_ROWS + 1, "the numbering band is gone");
    assert!(table.rows[0].is_header, "the wrapped band is the first row");
    assert_eq!(table.rows[0].cells[0].text, "Amount");
}

#[test]
fn a_drifting_leaf_track_is_not_one_row_band() {
    // Comparing each cell with its predecessor would accept 0, 0.9, 1.8; a track must agree with
    // the first cell's edges within the snap tolerance. ~keep
    let cells: Vec<TableCell> = [0.0, 0.9, 1.8]
        .into_iter()
        .enumerate()
        .map(|(column, drift)| stacked_cell("x", column, 1, 30.0 + drift, STACKED_WIDTH, true))
        .collect();
    assert!(super::super::single_title::row_tracks(&cells).is_none());
}

#[test]
fn a_stacked_title_reaches_the_table_with_both_header_bands() {
    let rows = stacked_rows("", true, false);
    let tables = detected(&stacked_rules(), &grid_spans(&rows));
    assert_eq!(tables.len(), 1, "one ruled table");
    let table = &tables[0];
    assert!(table.has_header, "the stacked title proves a header");
    assert_eq!(table.rows.len(), 4, "title, leaves, two body rows");
    assert_eq!((table.rows[0].cells[1].colspan, table.rows[0].cells[1].rowspan), (4, 1));
    assert_eq!(
        table.rows.iter().map(|row| row.is_header).collect::<Vec<_>>(),
        [true, true, false, false]
    );
    assert_eq!(
        table.rows[1]
            .cells
            .iter()
            .map(|cell| cell.text.as_str())
            .collect::<Vec<_>>(),
        ["Limit", "1", "2", "3", "4"]
    );
}
