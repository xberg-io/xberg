use super::*;
use crate::elements::PathOperation;
use crate::layout::text_block::{Color, FontWeight, TextSpan};
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
fn year_group_labels_over_quarter_leaves_become_headers() {
    let mut rows = standard_rows();
    rows[0].cells[1] = cell("2023", 1, 2, HEIGHT, true);
    rows[0].cells[2] = cell("2024", 3, 2, HEIGHT, true);
    assert!(retained(rows, &standard_rules()));
}

#[test]
fn a_blank_group_label_is_rejected() {
    let mut rows = standard_rows();
    rows[0].cells[1] = cell(" ", 1, 2, HEIGHT, true);
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

fn white_fill(x: f32, y: f32, width: f32, height: f32) -> PathContent {
    let mut path = PathContent::rect(x, y, width, height).with_fill(Color::white());
    path.stroke_color = None;
    path
}

#[test]
fn a_white_fill_painted_over_a_rule_hides_it() {
    let mut rules = standard_rules();
    rules.push(white_fill(0.0, -5.0, 100.0, 10.0));
    assert!(!retained(standard_rows(), &rules));
}

#[test]
fn a_white_fill_painted_before_a_rule_does_not_hide_it() {
    let mut rules = vec![white_fill(0.0, -5.0, 100.0, 10.0)];
    rules.extend(standard_rules());
    assert!(retained(standard_rows(), &rules));
}

#[test]
fn a_rule_drawn_only_as_a_thin_dark_fill_is_not_accepted() {
    let mut rules = standard_rules();
    let mut fill = PathContent::rect(0.0, 0.0, 100.0, 0.5).with_fill(Color::black());
    fill.stroke_color = None;
    rules[0] = fill;
    assert!(!retained(standard_rows(), &rules));
}

#[test]
fn a_group_bottom_three_points_off_the_leaf_top_is_rejected() {
    let mut rows = standard_rows();
    rows[0].cells[1] = cell("Revenue", 1, 2, HEIGHT + 3.0, true);
    let mut rules = standard_rules();
    rules.push(PathContent::line(20.0, HEIGHT + 3.0, 60.0, HEIGHT + 3.0));
    assert!(!retained(rows, &rules));
}

#[test]
fn a_group_right_edge_two_points_past_its_last_leaf_is_rejected() {
    let mut rows = standard_rows();
    rows[0].cells[1].bbox = Some(Rect::new(20.0, HEIGHT, 42.0, HEIGHT));
    assert!(!retained(rows, &standard_rules()));
}

#[test]
fn a_group_with_rowspan_two_is_rejected() {
    let mut rows = standard_rows();
    rows[0].cells[1].rowspan = 2;
    assert!(!retained(rows, &standard_rules()));
}

#[test]
fn a_group_span_overflowing_its_cell_is_rejected() {
    let mut rows = standard_rows();
    rows[0].cells[1].spans[0].bbox = Rect::new(19.0, HEIGHT + 1.0, 42.0, HEIGHT - 2.0);
    assert!(!retained(rows, &standard_rules()));
}

#[test]
fn an_already_flagged_header_row_is_left_alone() {
    let mut rows = standard_rows();
    rows[0].is_header = true;
    retain_grouped_leaf_header(&mut rows, &standard_rules(), COLUMNS);
    assert!(rows[0].is_header);
    assert!(!rows[1].is_header);
    assert!(rows[1].cells.iter().all(|cell| !cell.is_header));
}

#[test]
fn a_light_stroke_over_only_the_middle_of_the_leaf_rule_is_rejected() {
    let mut rules = standard_rules();
    rules.push(restyled(PathContent::line(40.0, 0.0, 60.0, 0.0), Color::white()));
    assert!(!retained(standard_rows(), &rules));
}

#[test]
fn a_pale_tinted_rule_is_not_visible() {
    let mut rules = standard_rules();
    rules[0] = restyled(rules[0].clone(), Color::new(1.0, 1.0, 0.9));
    assert!(!retained(standard_rows(), &rules));
}

#[test]
fn a_dotted_line_of_separate_sub_paths_is_not_a_solid_rule() {
    let mut rules = standard_rules();
    rules.remove(0);
    rules.extend((0..50).map(|dot| PathContent::line(dot as f32 * 2.0, 0.0, dot as f32 * 2.0 + 1.0, 0.0)));
    assert!(!retained(standard_rows(), &rules));
}

#[test]
fn a_fill_starting_a_hundredth_of_a_point_before_the_rule_does_not_hide_it() {
    let mut rules = standard_rules();
    rules.push(white_fill(0.0, -0.01, 100.0, 5.01));
    assert!(retained(standard_rows(), &rules));
}

#[test]
fn a_stroked_fill_with_its_geometric_edge_on_the_rule_does_not_hide_it() {
    let mut rules = standard_rules();
    rules.push(
        PathContent::rect(0.0, 0.0, 100.0, 10.0)
            .with_fill(Color::white())
            .with_stroke_width(4.0),
    );
    assert!(retained(standard_rows(), &rules));
}

#[test]
fn a_non_rectangular_fill_over_the_rule_does_not_hide_it() {
    let mut triangle = PathContent::from_operations(vec![
        PathOperation::MoveTo(0.0, -5.0),
        PathOperation::LineTo(100.0, -5.0),
        PathOperation::LineTo(50.0, 5.0),
        PathOperation::ClosePath,
    ])
    .with_fill(Color::white());
    triangle.stroke_color = None;
    let mut rules = standard_rules();
    rules.push(triangle);
    assert!(retained(standard_rows(), &rules));
}

fn unchanged_after_retaining(mut rows: Vec<TableRow>) {
    let before: Vec<bool> = rows.iter().map(|row| row.is_header).collect();
    let cells_before: Vec<Vec<bool>> = rows
        .iter()
        .map(|row| row.cells.iter().map(|cell| cell.is_header).collect())
        .collect();
    retain_grouped_leaf_header(&mut rows, &standard_rules(), COLUMNS);
    assert_eq!(rows.iter().map(|row| row.is_header).collect::<Vec<_>>(), before);
    assert_eq!(
        rows.iter()
            .map(|row| row.cells.iter().map(|cell| cell.is_header).collect::<Vec<_>>())
            .collect::<Vec<_>>(),
        cells_before
    );
}

#[test]
fn an_already_flagged_leaf_row_is_left_alone() {
    let mut rows = standard_rows();
    rows[1].is_header = true;
    unchanged_after_retaining(rows);
}

#[test]
fn an_already_flagged_first_body_row_is_left_alone() {
    let mut rows = standard_rows();
    rows[2].is_header = true;
    unchanged_after_retaining(rows);
}

#[test]
fn a_header_flagged_cell_in_any_band_is_left_alone() {
    for (row, column) in [(0, 1), (1, 2), (2, 3)] {
        let mut rows = standard_rows();
        rows[row].cells[column].is_header = true;
        unchanged_after_retaining(rows);
    }
}

#[test]
fn a_body_or_leaf_cell_that_is_not_plain_horizontal_text_is_rejected() {
    let spoilers: [fn(&mut TextSpan); 2] = [|span| span.rotation_degrees = 90.0, |span| span.wmode = 1];
    for spoil in spoilers {
        for (row, column) in [(1, 2), (2, 2)] {
            let mut rows = standard_rows();
            spoil(&mut rows[row].cells[column].spans[0]);
            assert!(!retained(rows, &standard_rules()), "row {row}");
        }
        let mut rows = standard_rows();
        spoil(&mut rows[0].cells[1].spans[0]);
        assert!(!retained(rows, &standard_rules()), "group");
    }
}

#[test]
fn a_leaf_or_body_cell_without_text_spans_or_bounds_is_rejected() {
    for row in [1, 2] {
        let mut rows = standard_rows();
        rows[row].cells[2].text = String::new();
        assert!(!retained(rows, &standard_rules()), "empty text, row {row}");

        let mut rows = standard_rows();
        rows[row].cells[2].spans.clear();
        assert!(!retained(rows, &standard_rules()), "no spans, row {row}");

        let mut rows = standard_rows();
        rows[row].cells[2].bbox = None;
        assert!(!retained(rows, &standard_rules()), "no bounds, row {row}");
    }
}

#[test]
fn a_leaf_span_overflowing_its_cell_is_rejected() {
    let mut rows = standard_rows();
    rows[1].cells[2].spans[0].bbox = Rect::new(39.0, 1.0, 22.0, HEIGHT - 2.0);
    assert!(!retained(rows, &standard_rules()));
}

#[test]
fn a_band_with_the_wrong_cell_count_is_rejected() {
    let mut rows = standard_rows();
    rows[2].cells.pop();
    assert!(!retained(rows, &standard_rules()));
    let mut rows = standard_rows();
    rows[1].cells.pop();
    assert!(!retained(rows, &standard_rules()));
}

#[test]
fn a_spanning_cell_in_the_leaf_band_is_rejected() {
    let mut rows = standard_rows();
    rows[1].cells[2].colspan = 2;
    assert!(!retained(rows, &standard_rules()));
}

#[test]
fn a_row_span_on_a_leaf_or_body_cell_is_rejected() {
    for row in [1, 2] {
        let mut rows = standard_rows();
        rows[row].cells[2].rowspan = 2;
        assert!(!retained(rows, &standard_rules()), "row {row}");
    }
}

#[test]
fn an_empty_first_group_must_be_a_plain_blank_cell() {
    let mut rows = standard_rows();
    rows[0].cells[0].rowspan = 2;
    assert!(!retained(rows, &standard_rules()));

    let mut rows = standard_rows();
    rows[0].cells[0].spans = cell("x", 0, 1, HEIGHT, false).spans;
    assert!(!retained(rows, &standard_rules()));

    let mut rows = standard_rows();
    rows[0].cells[0].text = "x".to_string();
    assert!(!retained(rows, &standard_rules()));
}

#[test]
fn an_empty_first_group_may_cover_several_label_columns() {
    let columns = 6;
    let groups = row(vec![
        cell("", 0, 2, HEIGHT, false),
        cell("Revenue", 2, 2, HEIGHT, true),
        cell("Costs", 4, 2, HEIGHT, true),
    ]);
    let leaf_band = row(["Item", "Unit", "Q1", "Q2", "Q1", "Q2"]
        .iter()
        .enumerate()
        .map(|(column, label)| cell(label, column, 1, 0.0, true))
        .collect());
    let body_band = row((0..columns)
        .map(|column| cell("1.5", column, 1, -HEIGHT, false))
        .collect());
    let mut rows = vec![groups, leaf_band, body_band];
    let rules = vec![
        PathContent::line(0.0, 0.0, 120.0, 0.0),
        PathContent::line(40.0, HEIGHT, 80.0, HEIGHT),
        PathContent::line(80.0, HEIGHT, 120.0, HEIGHT),
    ];
    retain_grouped_leaf_header(&mut rows, &rules, columns);
    assert!(rows[0].is_header && rows[1].is_header);
}

#[test]
fn a_second_group_only_spans_when_the_first_is_blank() {
    let mut rows = standard_rows();
    rows[0] = row(vec![cell("", 0, 1, HEIGHT, false), cell("Revenue", 1, 4, HEIGHT, true)]);
    let rules = vec![
        PathContent::line(0.0, 0.0, 100.0, 0.0),
        PathContent::line(20.0, HEIGHT, 100.0, HEIGHT),
    ];
    assert!(!retained(rows, &rules));
}

#[test]
fn a_negative_color_channel_is_not_a_dark_rule() {
    let mut rules = standard_rules();
    rules[0] = restyled(rules[0].clone(), Color::new(-0.5, 0.0, 0.0));
    assert!(!retained(standard_rows(), &rules));
}

#[test]
fn the_lightest_accepted_rule_color_is_pinned_at_a_tenth_below_white() {
    let mut rules = standard_rules();
    rules[0] = restyled(rules[0].clone(), Color::new(0.9, 0.9, 0.9));
    assert!(retained(standard_rows(), &rules));
    rules[0] = restyled(rules[0].clone(), Color::new(0.9, 0.9, 0.91));
    assert!(!retained(standard_rows(), &rules));
}

#[test]
fn a_hairline_rule_with_zero_width_is_visible() {
    let mut rules = standard_rules();
    rules[0] = rules[0].clone().with_stroke_width(0.0);
    assert!(retained(standard_rows(), &rules));
}

#[test]
fn a_rule_with_a_negative_or_non_finite_width_is_rejected() {
    for width in [-1.0, f32::NAN, f32::INFINITY] {
        let mut rules = standard_rules();
        rules[0] = rules[0].clone().with_stroke_width(width);
        assert!(!retained(standard_rows(), &rules), "width {width}");
    }
}

#[test]
fn segments_closer_than_a_tenth_of_a_point_join_into_one_rule() {
    let mut rules = standard_rules();
    rules[0] = PathContent::line(0.0, 0.0, 50.0, 0.0);
    rules.push(PathContent::line(50.05, 0.0, 100.0, 0.0));
    assert!(retained(standard_rows(), &rules));

    rules[3] = PathContent::line(50.3, 0.0, 100.0, 0.0);
    assert!(!retained(standard_rows(), &rules));
}

#[test]
fn a_thin_border_painted_across_a_rule_splits_it() {
    let mut rules = standard_rules();
    rules.push(white_fill(49.5, -5.0, 1.0, 10.0));
    assert!(!retained(standard_rows(), &rules));
}

fn grid_lines() -> Vec<PathContent> {
    let mut lines = Vec::new();
    for y in [-20.0, -10.0, 0.0, 10.0, 20.0] {
        let line = PathContent::line(0.0, y, 100.0, y);
        lines.push(if y < 0.0 { line.with_stroke_width(0.5) } else { line });
    }
    for x in [0.0, 20.0, 60.0, 100.0] {
        lines.push(PathContent::line(x, -20.0, x, 20.0));
    }
    for x in [40.0, 80.0] {
        lines.push(PathContent::line(x, -20.0, x, 10.0));
    }
    lines
}

fn grid_spans() -> Vec<TextSpan> {
    let mut spans = Vec::new();
    let mut bold = |text: &str, x: f32, y: f32, width: f32| {
        let mut span = create_test_span(text, x + 1.0, y + 1.0, width - 2.0, HEIGHT - 2.0);
        span.font_weight = FontWeight::Bold;
        spans.push(span);
    };
    bold("Revenue", 20.0, 10.0, 40.0);
    bold("Costs", 60.0, 10.0, 40.0);
    for (column, label) in ["Item", "Q1", "Q2", "Q1", "Q2"].iter().enumerate() {
        bold(label, column as f32 * WIDTH, 0.0, WIDTH);
    }
    for row in 0..2 {
        for column in 0..COLUMNS {
            spans.push(create_test_span(
                "1.5",
                column as f32 * WIDTH + 1.0,
                -HEIGHT * (row as f32 + 1.0) + 1.0,
                WIDTH - 2.0,
                HEIGHT - 2.0,
            ));
        }
    }
    spans
}

fn detected(lines: &[PathContent], spans: &[TextSpan]) -> Vec<Table> {
    let config = crate::structure::spatial_table_detector::TableDetectionConfig::default();
    crate::structure::spatial_table_detector::detect_intersections(spans, lines, &config, None)
}

fn header_flags(tables: &[Table]) -> Vec<bool> {
    assert_eq!(tables.len(), 1, "one table expected");
    tables[0].rows.iter().map(|row| row.is_header).collect()
}

#[test]
fn a_ruled_grid_with_grouped_bold_headers_reaches_the_table_with_both_bands_flagged() {
    let tables = detected(&grid_lines(), &grid_spans());
    assert_eq!(header_flags(&tables), [true, true, false, false]);
    assert!(tables[0].has_header);
    assert!(
        tables[0].rows[..2]
            .iter()
            .all(|row| row.cells.iter().all(|cell| cell.is_header))
    );
}

#[test]
fn the_same_grid_with_a_white_leaf_rule_keeps_no_header() {
    let mut lines = grid_lines();
    let leaf_rule = lines
        .iter()
        .position(|line| line.bbox.y == 0.0 && line.bbox.width > 90.0)
        .unwrap();
    lines[leaf_rule] = restyled(lines[leaf_rule].clone(), Color::white());
    let tables = detected(&lines, &grid_spans());
    assert_eq!(header_flags(&tables), [false, false, false, false]);
}

#[test]
fn a_numbering_row_above_the_header_bands_does_not_hide_them() {
    let mut lines = grid_lines();
    lines.push(PathContent::line(0.0, 30.0, 100.0, 30.0));
    for x in [0.0, 20.0, 40.0, 60.0, 80.0, 100.0] {
        lines.push(PathContent::line(x, 20.0, x, 30.0));
    }
    let mut spans = grid_spans();
    for column in 0..COLUMNS {
        spans.push(create_test_span(
            "1",
            column as f32 * WIDTH + 1.0,
            21.0,
            WIDTH - 2.0,
            HEIGHT - 2.0,
        ));
    }
    let tables = detected(&lines, &spans);
    assert_eq!(header_flags(&tables), [true, true, false, false]);
}

#[test]
fn an_empty_band_above_the_header_bands_does_not_hide_them() {
    let mut lines = grid_lines();
    lines.push(PathContent::line(0.0, 30.0, 100.0, 30.0));
    for x in [0.0, 20.0, 40.0, 60.0, 80.0, 100.0] {
        lines.push(PathContent::line(x, 20.0, x, 30.0));
    }
    let tables = detected(&lines, &grid_spans());
    let flags = header_flags(&tables);
    assert_eq!(&flags[flags.len() - 4..], [true, true, false, false]);
}

#[test]
fn a_grid_ruled_alike_on_every_row_keeps_no_header() {
    let mut lines = grid_lines();
    for line in &mut lines {
        *line = line.clone().with_stroke_width(1.0);
    }
    let tables = detected(&lines, &grid_spans());
    assert_eq!(header_flags(&tables), [false, false, false, false]);
}

fn body_rule_rows() -> (Vec<TableRow>, Vec<PathContent>) {
    (standard_rows(), standard_rules())
}

#[test]
fn a_body_rule_of_the_same_weight_and_colour_rejects_the_header() {
    let (rows, mut rules) = body_rule_rows();
    rules.push(PathContent::line(0.0, -HEIGHT, 100.0, -HEIGHT));
    assert!(!retained(rows, &rules));
}

#[test]
fn a_thinner_body_rule_keeps_the_header() {
    let (rows, mut rules) = body_rule_rows();
    rules.push(PathContent::line(0.0, -HEIGHT, 100.0, -HEIGHT).with_stroke_width(0.5));
    assert!(retained(rows, &rules));
}

#[test]
fn a_lighter_body_rule_keeps_the_header() {
    let (rows, mut rules) = body_rule_rows();
    rules.push(restyled(
        PathContent::line(0.0, -HEIGHT, 100.0, -HEIGHT),
        Color::new(0.5, 0.5, 0.5),
    ));
    assert!(retained(rows, &rules));
}

#[test]
fn a_partial_body_rule_does_not_reject_the_header() {
    let (rows, mut rules) = body_rule_rows();
    rules.push(PathContent::line(0.0, -HEIGHT, 30.0, -HEIGHT));
    assert!(retained(rows, &rules));
}

fn repeated_schema_rows() -> Vec<TableRow> {
    let mut rows = standard_rows();
    rows[1] = leaves(["Item", "Value", "Count", "Value", "Count"], true);
    rows
}

fn shaded_first_body() -> Vec<PathContent> {
    let mut fill = white_fill(0.0, -HEIGHT, COLUMNS as f32 * WIDTH, HEIGHT);
    fill.fill_color = Some(Color::new(0.9, 0.9, 0.9));
    vec![fill]
}

#[test]
fn repeated_group_schema_and_shaded_first_body_supply_independent_header_evidence() {
    assert!(retained(repeated_schema_rows(), &shaded_first_body()));
}

#[test]
fn repeated_schema_requires_full_first_body_shading_and_unshaded_headers() {
    assert!(!retained(repeated_schema_rows(), &[]));
    let mut partial = shaded_first_body();
    partial[0].bbox.width -= WIDTH;
    assert!(!retained(repeated_schema_rows(), &partial));
    let mut shaded_header = shaded_first_body();
    let mut header_fill = shaded_header[0].clone();
    header_fill.bbox.y = 0.0;
    shaded_header.push(header_fill);
    assert!(!retained(repeated_schema_rows(), &shaded_header));
}

#[test]
fn different_or_constant_leaf_schemas_do_not_supply_header_evidence() {
    let mut different = repeated_schema_rows();
    different[1].cells[4].text = "Other".into();
    assert!(!retained(different, &shaded_first_body()));
    let mut constant = repeated_schema_rows();
    for cell in &mut constant[1].cells[1..] {
        cell.text = "Value".into();
    }
    assert!(!retained(constant, &shaded_first_body()));
}

#[test]
fn repeated_schema_does_not_override_body_style_or_source_geometry() {
    let mut bold = repeated_schema_rows();
    bold[2] = body(true);
    assert!(!retained(bold, &shaded_first_body()));
    for column in 0..COLUMNS {
        let mut shifted = repeated_schema_rows();
        shifted[2].cells[column].bbox.as_mut().unwrap().x += 2.0;
        assert!(!retained(shifted, &shaded_first_body()));
        let mut separated = repeated_schema_rows();
        separated[2].cells[column].bbox.as_mut().unwrap().y -= 2.0;
        assert!(!retained(separated, &shaded_first_body()));
    }
}

#[test]
fn later_rectangular_paint_withholds_the_shading_proof() {
    let mut erased = shaded_first_body();
    erased.push(white_fill(0.0, -HEIGHT, COLUMNS as f32 * WIDTH, HEIGHT));
    assert!(!retained(repeated_schema_rows(), &erased));
    erased.extend(shaded_first_body());
    assert!(retained(repeated_schema_rows(), &erased));
}

#[test]
fn repeated_schema_requires_an_ascii_digit_in_each_body_value_after_column_zero() {
    let mut rows = repeated_schema_rows();
    rows[2].cells[1].text = "Note".into();
    assert!(!retained(rows, &shaded_first_body()));
}

#[test]
fn possible_header_shading_remains_ambiguous_after_partial_overpainting() {
    let mut paths = shaded_first_body();
    let mut leaf_fill = paths[0].clone();
    leaf_fill.bbox.y = 0.0;
    paths.push(leaf_fill);
    paths.push(white_fill(0.0, 0.0, 1.0, 1.0));
    assert!(!retained(repeated_schema_rows(), &paths));
}

#[test]
fn the_same_leaf_and_body_rule_vetoes_independent_shading_evidence() {
    let mut paths = shaded_first_body();
    paths.push(PathContent::line(0.0, 0.0, COLUMNS as f32 * WIDTH, 0.0));
    paths.push(PathContent::line(0.0, -HEIGHT, COLUMNS as f32 * WIDTH, -HEIGHT));
    assert!(!retained(repeated_schema_rows(), &paths));
}

#[test]
fn numeric_only_leaf_schemas_need_visible_rules() {
    let mut rows = repeated_schema_rows();
    rows[1] = leaves(["Item", "2023", "2024", "2023", "2024"], true);
    assert!(complete_source_band(&rows[1], COLUMNS, true));
    assert!(grouped_leaves(&rows[0], &rows[1]).is_some());
    assert!(!retained(rows, &shaded_first_body()));
}

#[test]
fn body_column_alignment_is_required_even_when_text_moves_with_its_cell() {
    let mut rows = repeated_schema_rows();
    let value = &mut rows[2].cells[2];
    value.bbox.as_mut().unwrap().x += 2.0;
    for span in &mut value.spans {
        span.bbox.x += 2.0;
    }
    assert!(complete_source_band(&rows[2], COLUMNS, false));
    let shading = shaded_first_body();
    assert!(cell_is_shaded(&rows[2].cells[2], &shading_rectangles(shading.iter())));
    assert!(!retained(rows, &shading));
}
