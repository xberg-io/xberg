use super::*;
use crate::layout::text_block::{Color, FontWeight};
use crate::structure::spatial_table_detector::tests::create_test_span;

const COLUMNS: usize = 5;
const WIDTH: f32 = 20.0;
const HEIGHT: f32 = 10.0;

fn cell(text: &str, column: usize, colspan: u32, y: f32, bold: bool) -> TableCell {
    let x = column as f32 * WIDTH;
    let width = colspan as f32 * WIDTH;
    let spans = if text.is_empty() {
        Vec::new()
    } else {
        let mut span = create_test_span(text, x + 1.0, y + 1.0, width - 2.0, HEIGHT - 2.0);
        if bold {
            span.font_weight = FontWeight::Bold;
        }
        vec![span]
    };
    TableCell {
        text: text.to_string(),
        spans,
        colspan,
        rowspan: 1,
        mcids: Vec::new(),
        bbox: Some(Rect::new(x, y, width, HEIGHT)),
        is_header: false,
    }
}

fn row(cells: Vec<TableCell>) -> TableRow {
    TableRow {
        cells,
        is_header: false,
    }
}

fn leaves(labels: [&str; COLUMNS], bold: bool) -> TableRow {
    row(labels
        .iter()
        .enumerate()
        .map(|(column, label)| cell(label, column, 1, 0.0, bold))
        .collect())
}

fn body(bold: bool) -> TableRow {
    row((0..COLUMNS)
        .map(|column| cell("1.5", column, 1, -HEIGHT, bold))
        .collect())
}

fn two_groups() -> TableRow {
    row(vec![
        cell("", 0, 1, HEIGHT, false),
        cell("Revenue", 1, 2, HEIGHT, true),
        cell("Costs", 3, 2, HEIGHT, true),
    ])
}

fn standard_rows() -> Vec<TableRow> {
    vec![
        two_groups(),
        leaves(["Item", "Q1", "Q2", "Q1", "Q2"], true),
        body(false),
    ]
}

fn standard_rules() -> Vec<PathContent> {
    vec![
        PathContent::line(0.0, 0.0, 100.0, 0.0),
        PathContent::line(20.0, HEIGHT, 60.0, HEIGHT),
        PathContent::line(60.0, HEIGHT, 100.0, HEIGHT),
    ]
}

fn retained(mut rows: Vec<TableRow>, paths: &[PathContent]) -> bool {
    retain_grouped_leaf_header(&mut rows, paths, COLUMNS);
    let flags: Vec<bool> = rows.iter().map(|row| row.is_header).collect();
    assert!(
        flags == [true, true, false] || flags == [false, false, false],
        "header roles must be set on both bands or neither: {flags:?}"
    );
    assert!(rows[0].cells.iter().all(|cell| cell.is_header == flags[0]));
    assert!(rows[1].cells.iter().all(|cell| cell.is_header == flags[1]));
    flags[0]
}

fn restyled(mut path: PathContent, color: Color) -> PathContent {
    path.stroke_color = Some(color);
    path
}

#[test]
fn two_ruled_bold_groups_over_bold_leaves_become_headers() {
    assert!(retained(standard_rows(), &standard_rules()));
}

#[test]
fn numeric_year_leaves_still_qualify() {
    let mut rows = standard_rows();
    rows[1] = leaves(["Item", "2023", "2024", "2023", "2024"], true);
    assert!(retained(rows, &standard_rules()));
}

#[test]
fn a_non_bold_leaf_band_is_not_a_header() {
    let mut rows = standard_rows();
    rows[1] = leaves(["Item", "Q1", "Q2", "Q1", "Q2"], false);
    assert!(!retained(rows, &standard_rules()));
}

#[test]
fn a_bold_first_body_row_is_not_a_header() {
    let mut rows = standard_rows();
    rows[2] = body(true);
    assert!(!retained(rows, &standard_rules()));
}

#[test]
fn a_group_edge_two_points_off_the_leaf_edges_is_rejected() {
    let mut rows = standard_rows();
    rows[0].cells[1].bbox = Some(Rect::new(22.0, HEIGHT, 38.0, HEIGHT));
    assert!(!retained(rows, &standard_rules()));
}

#[test]
fn a_dashed_group_rule_is_rejected() {
    let mut rules = standard_rules();
    rules[1].dash_pattern = Some((vec![3.0, 2.0], 0.0));
    assert!(!retained(standard_rows(), &rules));
}

#[test]
fn a_white_leaf_rule_is_rejected() {
    let mut rules = standard_rules();
    rules[0] = restyled(rules[0].clone(), Color::white());
    assert!(!retained(standard_rows(), &rules));
}

#[test]
fn a_white_stroke_painted_over_a_rule_hides_it() {
    let mut rules = standard_rules();
    rules.push(restyled(PathContent::line(0.0, 0.0, 100.0, 0.0), Color::white()));
    assert!(!retained(standard_rows(), &rules));
}

#[test]
fn a_white_stroke_painted_before_a_rule_does_not_hide_it() {
    let mut rules = vec![restyled(PathContent::line(0.0, 0.0, 100.0, 0.0), Color::white())];
    rules.extend(standard_rules());
    assert!(retained(standard_rows(), &rules));
}

#[test]
fn a_missing_group_rule_is_rejected() {
    let mut rules = standard_rules();
    rules.remove(2);
    assert!(!retained(standard_rows(), &rules));
}

#[test]
fn an_empty_group_after_the_first_is_rejected() {
    let mut rows = standard_rows();
    rows[0] = row(vec![
        cell("Revenue", 0, 2, HEIGHT, true),
        cell("Costs", 2, 2, HEIGHT, true),
        cell("", 4, 1, HEIGHT, false),
    ]);
    let rules = vec![
        PathContent::line(0.0, 0.0, 100.0, 0.0),
        PathContent::line(0.0, HEIGHT, 40.0, HEIGHT),
        PathContent::line(40.0, HEIGHT, 80.0, HEIGHT),
    ];
    assert!(!retained(rows, &rules));
}

#[test]
fn only_one_spanning_group_is_rejected() {
    let mut rows = standard_rows();
    rows[0] = row(vec![
        cell("", 0, 1, HEIGHT, false),
        cell("Revenue", 1, 2, HEIGHT, true),
        cell("Costs", 3, 1, HEIGHT, true),
        cell("Other", 4, 1, HEIGHT, true),
    ]);
    assert!(!retained(rows, &standard_rules()));
}

#[test]
fn a_group_spanning_more_columns_than_remain_is_rejected() {
    let mut rows = standard_rows();
    rows[0].cells[2] = cell("Costs", 3, 3, HEIGHT, true);
    assert!(!retained(rows, &standard_rules()));
}

#[test]
fn numeric_group_labels_do_not_make_a_header() {
    let mut rows = standard_rows();
    rows[0].cells[1] = cell("2023", 1, 2, HEIGHT, true);
    rows[0].cells[2] = cell("2024", 3, 2, HEIGHT, true);
    assert!(!retained(rows, &standard_rules()));
}

#[test]
fn collinear_segments_together_cover_a_leaf_rule() {
    let mut rules = standard_rules();
    rules[0] = PathContent::line(0.0, 0.0, 50.0, 0.0);
    rules.push(PathContent::line(50.0, 0.0, 100.0, 0.0));
    assert!(retained(standard_rows(), &rules));
}

#[test]
fn a_gap_between_collinear_segments_leaves_the_rule_uncovered() {
    let mut rules = standard_rules();
    rules[0] = PathContent::line(0.0, 0.0, 45.0, 0.0);
    rules.push(PathContent::line(55.0, 0.0, 100.0, 0.0));
    assert!(!retained(standard_rows(), &rules));
}

#[test]
fn unrelated_page_paths_do_not_disable_the_rule() {
    let mut rules: Vec<PathContent> = (0..=MAX_GROUPED_HEADER_PATHS)
        .map(|index| PathContent::line(500.0, 300.0 + index as f32, 600.0, 300.0 + index as f32))
        .collect();
    rules.extend(standard_rules());
    assert!(retained(standard_rows(), &rules));
}

#[test]
fn more_than_the_cap_of_paths_near_the_header_is_rejected() {
    let mut rules: Vec<PathContent> = (0..=MAX_GROUPED_HEADER_PATHS)
        .map(|index| PathContent::line(0.0, 5.0 + index as f32 * 0.01, 100.0, 5.0 + index as f32 * 0.01))
        .collect();
    rules.extend(standard_rules());
    assert!(!retained(standard_rows(), &rules));
}
